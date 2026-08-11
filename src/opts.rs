use std::path::PathBuf;
use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    /// Interactive terminal UI (Ratatui treemap)
    Tui,
    /// Human-readable ASCII treemap and text summary
    Text,
    /// Machine-readable JSON output
    Json,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, Copy)]
pub enum SortMetric {
    /// Sort by disk size footprint (bytes)
    Size,
    /// Sort by derive macro expansion weight
    Macro,
    /// Sort by generic monomorphization cost
    Monomorph,
    /// Sort by combined bloat score
    Total,
}

impl std::fmt::Display for SortMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortMetric::Size => write!(f, "Size"),
            SortMetric::Macro => write!(f, "Macro Weight"),
            SortMetric::Monomorph => write!(f, "Monomorph Cost"),
            SortMetric::Total => write!(f, "Total Bloat"),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "cargo-light",
    author = "Antigravity <agy@google.com>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Rust binary dependency bloat analyzer with interactive Treemap visualizer, derive macro weight & monomorphization cost analysis",
    long_about = None
)]
pub struct Opts {
    /// Path to Cargo.toml manifest file [default: ./Cargo.toml]
    #[arg(short, long, value_name = "PATH")]
    pub manifest_path: Option<PathBuf>,

    /// Limit the number of top dependencies displayed in text/json mode
    #[arg(short, long, value_name = "N")]
    pub top: Option<usize>,

    /// Output format: tui, text, or json [default: tui if interactive terminal, else text]
    #[arg(short, long, value_enum)]
    pub format: Option<OutputFormat>,

    /// Display actionable prune and replacement suggestions for bloat reduction
    #[arg(short = 'p', long)]
    pub prune_suggestions: bool,

    /// Primary metric to sort dependencies by (size, macro, monomorph, total) [default: size]
    #[arg(short = 's', long, value_enum, default_value_t = SortMetric::Size)]
    pub sort_by: SortMetric,

    /// Analyze all workspace packages recursively
    #[arg(short, long)]
    pub workspace: bool,

    /// Offline mode: avoid cargo network updates and rely on local metadata/cache
    #[arg(long)]
    pub offline: bool,

    /// Filter dependencies by name substring
    #[arg(short = 'F', long)]
    pub filter: Option<String>,
}
