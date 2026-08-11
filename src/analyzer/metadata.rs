use anyhow::{Context, Result};
use cargo_metadata::{MetadataCommand, Node, Package, PackageId};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analyzer::prune::{find_duplicates, generate_prune_suggestions};
use crate::analyzer::source::{
    analyze_source_dir, calculate_derive_macro_weight, calculate_monomorphization_cost,
};
use crate::model::{BloatAnalysisReport, CrateMetrics};
use crate::opts::Opts;

pub struct Analyzer {
    opts: Opts,
}

impl Analyzer {
    pub fn new(opts: Opts) -> Self {
        Self { opts }
    }

    pub fn analyze(&self) -> Result<BloatAnalysisReport> {
        let manifest_path = self.opts.manifest_path.clone().unwrap_or_else(|| PathBuf::from("Cargo.toml"));
        
        let mut cmd = MetadataCommand::new();
        cmd.manifest_path(&manifest_path);

        if self.opts.offline {
            cmd.other_options(vec!["--offline".to_string()]);
        }

        let metadata = cmd
            .exec()
            .with_context(|| format!("Failed to execute cargo metadata for manifest path: {}", manifest_path.display()))?;

        let workspace_members: HashSet<PackageId> = metadata.workspace_members.iter().cloned().collect();
        let resolve = metadata
            .resolve
            .as_ref()
            .context("No resolve graph found in Cargo metadata (is the project built/locked?)")?;

        let root_id = resolve.root.clone().unwrap_or_else(|| {
            workspace_members
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| metadata.packages[0].id.clone())
        });

        // Build package lookup maps
        let package_map: HashMap<PackageId, &Package> =
            metadata.packages.iter().map(|p| (p.id.clone(), p)).collect();
        let node_map: HashMap<PackageId, &Node> =
            resolve.nodes.iter().map(|n| (n.id.clone(), n)).collect();

        // Calculate dependency distance/depths and direct dependencies from root
        let root_node = node_map.get(&root_id);
        let direct_dep_ids: HashSet<PackageId> = root_node
            .map(|n| n.dependencies.iter().cloned().collect())
            .unwrap_or_default();

        let mut depth_map: HashMap<PackageId, usize> = HashMap::new();
        depth_map.insert(root_id.clone(), 0);

        // BFS depth calculation
        let mut queue = vec![(root_id.clone(), 0)];
        let mut visited = HashSet::new();
        visited.insert(root_id.clone());

        let mut dependents_map: HashMap<PackageId, Vec<String>> = HashMap::new();
        let mut dependencies_map: HashMap<PackageId, Vec<String>> = HashMap::new();

        while let Some((curr_id, curr_depth)) = queue.pop() {
            if let Some(node) = node_map.get(&curr_id) {
                let curr_pkg_name = package_map
                    .get(&curr_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();

                for dep_id in &node.dependencies {
                    if let Some(dep_pkg) = package_map.get(dep_id) {
                        dependencies_map
                            .entry(curr_id.clone())
                            .or_default()
                            .push(dep_pkg.name.clone());

                        dependents_map
                            .entry(dep_id.clone())
                            .or_default()
                            .push(curr_pkg_name.clone());
                    }

                    if !visited.contains(dep_id) {
                        visited.insert(dep_id.clone());
                        depth_map.insert(dep_id.clone(), curr_depth + 1);
                        queue.push((dep_id.clone(), curr_depth + 1));
                    }
                }
            }
        }

        // Analyze each package
        let mut crate_metrics = Vec::new();
        let mut total_size_bytes: u64 = 0;
        let mut total_code_lines: u64 = 0;

        for (pkg_id, pkg) in &package_map {
            // Filter out root itself if we are analyzing dependencies, or include if workspace
            let is_workspace = workspace_members.contains(pkg_id);
            let _is_root = *pkg_id == root_id;
            let is_direct = direct_dep_ids.contains(pkg_id);

            // Locate source directory
            let manifest_path_buf = pkg.manifest_path.as_std_path().to_path_buf();
            let source_dir = manifest_path_buf.parent().map(|p| p.to_path_buf());

            let is_proc_macro = pkg.targets.iter().any(|t| t.kind.iter().any(|k| k == "proc-macro"));

            let src_analysis = if let Some(ref dir) = source_dir {
                analyze_source_dir(dir, is_proc_macro)
            } else {
                analyze_source_dir(Path::new(""), is_proc_macro)
            };

            // Count transitive macro deps
            let direct_deps = dependencies_map.get(pkg_id).cloned().unwrap_or_default();
            let macro_deps_count = direct_deps
                .iter()
                .filter(|d| *d == "syn" || *d == "quote" || *d == "proc-macro2" || *d == "serde_derive" || d.contains("macro") || d.contains("derive"))
                .count();

            let derive_macro_weight = calculate_derive_macro_weight(
                src_analysis.derive_count,
                src_analysis.is_proc_macro,
                macro_deps_count,
                src_analysis.code_lines,
            );

            let monomorphization_cost = calculate_monomorphization_cost(
                src_analysis.generic_fn_count,
                src_analysis.code_lines,
                src_analysis.size_bytes,
            );

            // Bloat score combining size, macro weight, and monomorph cost
            let size_factor = (src_analysis.size_bytes as f64 / 500_000.0).min(40.0);
            let bloat_score = (size_factor + derive_macro_weight * 0.3 + monomorphization_cost * 0.3).min(100.0);

            let enabled_features = node_map
                .get(pkg_id)
                .map(|n| n.features.clone())
                .unwrap_or_default();

            let depth = depth_map.get(pkg_id).cloned().unwrap_or(99);

            total_size_bytes += src_analysis.size_bytes;
            total_code_lines += src_analysis.code_lines;

            crate_metrics.push(CrateMetrics {
                name: pkg.name.clone(),
                version: pkg.version.to_string(),
                package_id: pkg_id.to_string(),
                manifest_path: manifest_path_buf,
                source_path: source_dir,
                size_bytes: src_analysis.size_bytes,
                code_lines: src_analysis.code_lines,
                derive_macro_weight,
                derive_count: src_analysis.derive_count,
                is_proc_macro,
                monomorphization_cost,
                generic_fn_count: src_analysis.generic_fn_count,
                is_direct_dep: is_direct,
                is_workspace_member: is_workspace,
                dependencies: direct_deps,
                dependents: dependents_map.get(pkg_id).cloned().unwrap_or_default(),
                depth,
                features_enabled: enabled_features,
                bloat_score,
            });
        }

        // Apply filter if specified
        if let Some(ref filter_term) = self.opts.filter {
            let filter_lower = filter_term.to_lowercase();
            crate_metrics.retain(|m| m.name.to_lowercase().contains(&filter_lower));
        }

        // Sort crate metrics based on user option
        match self.opts.sort_by {
            crate::opts::SortMetric::Size => crate_metrics.sort_by_key(|b| std::cmp::Reverse(b.size_bytes)),
            crate::opts::SortMetric::Macro => crate_metrics.sort_by(|a, b| b.derive_macro_weight.partial_cmp(&a.derive_macro_weight).unwrap_or(std::cmp::Ordering::Equal)),
            crate::opts::SortMetric::Monomorph => crate_metrics.sort_by(|a, b| b.monomorphization_cost.partial_cmp(&a.monomorphization_cost).unwrap_or(std::cmp::Ordering::Equal)),
            crate::opts::SortMetric::Total => crate_metrics.sort_by(|a, b| b.bloat_score.partial_cmp(&a.bloat_score).unwrap_or(std::cmp::Ordering::Equal)),
        }

        let duplicates = find_duplicates(&crate_metrics);
        let prune_suggestions = generate_prune_suggestions(&crate_metrics, &duplicates);

        let root_name = package_map
            .get(&root_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "workspace".to_string());

        let total_deps = crate_metrics.len();
        let direct_deps_count = crate_metrics.iter().filter(|c| c.is_direct_dep).count();

        let avg_macro_weight = if total_deps > 0 {
            crate_metrics.iter().map(|c| c.derive_macro_weight).sum::<f64>() / (total_deps as f64)
        } else {
            0.0
        };

        let avg_monomorph_cost = if total_deps > 0 {
            crate_metrics.iter().map(|c| c.monomorphization_cost).sum::<f64>() / (total_deps as f64)
        } else {
            0.0
        };

        Ok(BloatAnalysisReport {
            workspace_name: metadata.workspace_root.file_name().unwrap_or("rust-project").to_string(),
            root_package: root_name,
            target_directory: metadata.target_directory.as_std_path().to_path_buf(),
            total_dependencies: total_deps,
            direct_dependencies: direct_deps_count,
            total_size_bytes,
            total_code_lines,
            avg_macro_weight,
            avg_monomorph_cost,
            crate_metrics,
            duplicates,
            prune_suggestions,
        })
    }
}
