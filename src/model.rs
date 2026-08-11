use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetrics {
    /// Package name (e.g. "serde")
    pub name: String,
    /// Package version (e.g. "1.0.197")
    pub version: String,
    /// Unique Package ID from cargo metadata
    pub package_id: String,
    /// Path to Cargo.toml for this package
    pub manifest_path: PathBuf,
    /// Source directory path (e.g. ~/.cargo/registry/src/... or local workspace path)
    pub source_path: Option<PathBuf>,
    /// Calculated disk size footprint in bytes (source + estimated compiled code)
    pub size_bytes: u64,
    /// Lines of Rust code (.rs files)
    pub code_lines: u64,
    /// Derive macro expansion weight score (0.0 to 100.0)
    pub derive_macro_weight: f64,
    /// Number of `#[derive(...)]` macro usages found
    pub derive_count: usize,
    /// True if package is a proc-macro crate
    pub is_proc_macro: bool,
    /// Generic monomorphization cost score (0.0 to 100.0)
    pub monomorphization_cost: f64,
    /// Count of generic function / impl declarations `<T>`
    pub generic_fn_count: usize,
    /// True if direct dependency of root crate
    pub is_direct_dep: bool,
    /// True if package is part of current workspace
    pub is_workspace_member: bool,
    /// List of direct dependency crate names
    pub dependencies: Vec<String>,
    /// List of crate names dependent on this crate
    pub dependents: Vec<String>,
    /// Distance from root package in dependency tree
    pub depth: usize,
    /// Enabled feature flags for this package
    pub features_enabled: Vec<String>,
    /// Combined bloat score (0.0 to 100.0)
    pub bloat_score: f64,
}

impl CrateMetrics {
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size_bytes)
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateInfo {
    pub name: String,
    pub versions: Vec<String>,
    pub wasted_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruneCategory {
    DuplicateVersion,
    HeavyProcMacro,
    HeavyMonomorphization,
    ReplaceableAlternative,
    BloatedFeatures,
}

impl std::fmt::Display for PruneCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PruneCategory::DuplicateVersion => write!(f, "Duplicate Version"),
            PruneCategory::HeavyProcMacro => write!(f, "Heavy Proc-Macro"),
            PruneCategory::HeavyMonomorphization => write!(f, "Heavy Monomorphization"),
            PruneCategory::ReplaceableAlternative => write!(f, "Lighter Alternative Available"),
            PruneCategory::BloatedFeatures => write!(f, "Bloated Features"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceLevel::High => write!(f, "High"),
            ConfidenceLevel::Medium => write!(f, "Medium"),
            ConfidenceLevel::Low => write!(f, "Low"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneSuggestion {
    pub crate_name: String,
    pub category: PruneCategory,
    pub title: String,
    pub description: String,
    pub potential_savings_bytes: u64,
    pub confidence: ConfidenceLevel,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloatAnalysisReport {
    pub workspace_name: String,
    pub root_package: String,
    pub target_directory: PathBuf,
    pub total_dependencies: usize,
    pub direct_dependencies: usize,
    pub total_size_bytes: u64,
    pub total_code_lines: u64,
    pub avg_macro_weight: f64,
    pub avg_monomorph_cost: f64,
    pub crate_metrics: Vec<CrateMetrics>,
    pub duplicates: Vec<DuplicateInfo>,
    pub prune_suggestions: Vec<PruneSuggestion>,
}

impl BloatAnalysisReport {
    pub fn get_metrics_by_name(&self, name: &str) -> Option<&CrateMetrics> {
        self.crate_metrics.iter().find(|m| m.name == name)
    }

    pub fn formatted_total_size(&self) -> String {
        format_bytes(self.total_size_bytes)
    }
}
