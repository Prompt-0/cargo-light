use crate::model::{
    ConfidenceLevel, CrateMetrics, DuplicateInfo, PruneCategory, PruneSuggestion,
};
use std::collections::HashMap;

pub fn find_duplicates(crates: &[CrateMetrics]) -> Vec<DuplicateInfo> {
    let mut by_name: HashMap<String, Vec<&CrateMetrics>> = HashMap::new();
    for c in crates {
        by_name.entry(c.name.clone()).or_default().push(c);
    }

    let mut duplicates = Vec::new();
    for (name, list) in by_name {
        if list.len() > 1 {
            let mut versions: Vec<String> = list.iter().map(|c| c.version.clone()).collect();
            versions.sort();
            versions.dedup();

            if versions.len() > 1 {
                // Calculate wasted bytes by duplicate versions (all versions except smallest)
                let mut sizes: Vec<u64> = list.iter().map(|c| c.size_bytes).collect();
                sizes.sort();
                let wasted_bytes: u64 = sizes.iter().skip(1).sum();

                duplicates.push(DuplicateInfo {
                    name,
                    versions,
                    wasted_bytes,
                });
            }
        }
    }

    duplicates.sort_by_key(|b| std::cmp::Reverse(b.wasted_bytes));
    duplicates
}

pub fn generate_prune_suggestions(
    crates: &[CrateMetrics],
    duplicates: &[DuplicateInfo],
) -> Vec<PruneSuggestion> {
    let mut suggestions = Vec::new();

    // 1. Duplicate dependency version suggestions
    for dup in duplicates {
        let est_savings = dup.wasted_bytes.max(250_000);
        suggestions.push(PruneSuggestion {
            crate_name: dup.name.clone(),
            category: PruneCategory::DuplicateVersion,
            title: format!("Consolidate duplicate versions of '{}'", dup.name),
            description: format!(
                "Multiple versions detected: {}. Use 'cargo tree -d' and update Cargo.toml or run 'cargo update -p {}' to consolidate.",
                dup.versions.join(", "),
                dup.name
            ),
            potential_savings_bytes: est_savings,
            confidence: ConfidenceLevel::High,
            replacement: Some(format!("Unify onto version {}", dup.versions.last().unwrap_or(&"latest".to_string()))),
        });
    }

    // 2. Heavy proc-macro and alternative recommendations
    for c in crates {
        // Heavy proc-macro crates
        if (c.is_proc_macro || c.name == "syn" || c.name == "quote" || c.name == "proc-macro2") && c.is_direct_dep {
            suggestions.push(PruneSuggestion {
                crate_name: c.name.clone(),
                category: PruneCategory::HeavyProcMacro,
                title: format!("Proc-macro dependency '{}' increases compile time", c.name),
                description: format!(
                    "'{}' is a direct proc-macro dependency (Macro weight: {:.1}). Consider replacing with declarative macros or manual trait implementations.",
                    c.name, c.derive_macro_weight
                ),
                potential_savings_bytes: c.size_bytes + 500_000,
                confidence: ConfidenceLevel::Medium,
                replacement: None,
            });
        }

        // Alternative suggestions for well-known heavy crates
        match c.name.as_str() {
            "reqwest" if c.is_direct_dep => {
                suggestions.push(PruneSuggestion {
                    crate_name: "reqwest".to_string(),
                    category: PruneCategory::ReplaceableAlternative,
                    title: "Consider replacing 'reqwest' with 'ureq' or 'minireq'".to_string(),
                    description: "'reqwest' brings in Tokio, Hyper, and OpenSSL/Rustls dependencies. 'ureq' is synchronous and drastically smaller for simple HTTP requests.".to_string(),
                    potential_savings_bytes: c.size_bytes + 2_500_000,
                    confidence: ConfidenceLevel::High,
                    replacement: Some("ureq".to_string()),
                });
            }
            "serde_json" if c.is_direct_dep => {
                suggestions.push(PruneSuggestion {
                    crate_name: "serde_json".to_string(),
                    category: PruneCategory::ReplaceableAlternative,
                    title: "Consider 'miniserde' or manual parsing for light JSON".to_string(),
                    description: "'serde_json' has generic monomorphization overhead. For small CLI utilities, 'miniserde' or custom deserialization reduces code size.".to_string(),
                    potential_savings_bytes: c.size_bytes + 400_000,
                    confidence: ConfidenceLevel::Medium,
                    replacement: Some("miniserde".to_string()),
                });
            }
            "thiserror" if c.is_direct_dep => {
                suggestions.push(PruneSuggestion {
                    crate_name: "thiserror".to_string(),
                    category: PruneCategory::ReplaceableAlternative,
                    title: "Replace 'thiserror' derive macro with manual Display / Error impls".to_string(),
                    description: "Implementing `std::fmt::Display` and `std::error::Error` manually eliminates proc-macro expand overhead for small error enums.".to_string(),
                    potential_savings_bytes: c.size_bytes + 300_000,
                    confidence: ConfidenceLevel::High,
                    replacement: Some("Manual std::fmt::Display impl".to_string()),
                });
            }
            "regex" if c.is_direct_dep => {
                suggestions.push(PruneSuggestion {
                    crate_name: "regex".to_string(),
                    category: PruneCategory::ReplaceableAlternative,
                    title: "Consider 'regex-lite' or 'memchr' string scanning".to_string(),
                    description: "'regex' builds large NFA state tables. 'regex-lite' or simple string methods reduce binary size significantly.".to_string(),
                    potential_savings_bytes: c.size_bytes + 1_200_000,
                    confidence: ConfidenceLevel::Medium,
                    replacement: Some("regex-lite".to_string()),
                });
            }
            _ => {}
        }

        // Heavy Monomorphization cost
        if c.monomorphization_cost > 60.0 && c.is_direct_dep {
            suggestions.push(PruneSuggestion {
                crate_name: c.name.clone(),
                category: PruneCategory::HeavyMonomorphization,
                title: format!("High generic monomorphization bloat in '{}'", c.name),
                description: format!(
                    "'{}' has {} generic item declarations (Monomorph cost: {:.1}). Use dynamic dispatch (`&dyn Trait` or `Box<dyn Trait>`) to reduce generic code duplication.",
                    c.name, c.generic_fn_count, c.monomorphization_cost
                ),
                potential_savings_bytes: (c.size_bytes as f64 * 0.4) as u64,
                confidence: ConfidenceLevel::Medium,
                replacement: Some("Use dynamic dispatch (dyn Trait)".to_string()),
            });
        }
    }

    suggestions.sort_by_key(|b| std::cmp::Reverse(b.potential_savings_bytes));
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn mock_crate(name: &str, ver: &str, size: u64, is_direct: bool) -> CrateMetrics {
        CrateMetrics {
            name: name.to_string(),
            version: ver.to_string(),
            package_id: format!("{} {}", name, ver),
            manifest_path: PathBuf::from("/mock/Cargo.toml"),
            source_path: None,
            size_bytes: size,
            code_lines: 100,
            derive_macro_weight: 10.0,
            derive_count: 2,
            is_proc_macro: false,
            monomorphization_cost: 15.0,
            generic_fn_count: 5,
            is_direct_dep: is_direct,
            is_workspace_member: false,
            dependencies: vec![],
            dependents: vec![],
            depth: 1,
            features_enabled: vec![],
            bloat_score: 20.0,
        }
    }

    #[test]
    fn test_find_duplicates() {
        let crates = vec![
            mock_crate("syn", "1.0.100", 500_000, false),
            mock_crate("syn", "2.0.40", 600_000, false),
            mock_crate("serde", "1.0.197", 300_000, true),
        ];

        let dups = find_duplicates(&crates);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].name, "syn");
        assert_eq!(dups[0].versions.len(), 2);
    }

    #[test]
    fn test_generate_prune_suggestions() {
        let crates = vec![
            mock_crate("reqwest", "0.11.0", 2_000_000, true),
            mock_crate("thiserror", "1.0.50", 150_000, true),
        ];
        let dups = vec![];

        let suggestions = generate_prune_suggestions(&crates, &dups);
        assert!(suggestions.iter().any(|s| s.crate_name == "reqwest"));
        assert!(suggestions.iter().any(|s| s.crate_name == "thiserror"));
    }
}
