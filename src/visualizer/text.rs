use crate::model::BloatAnalysisReport;
use crate::opts::{Opts, SortMetric};
use crate::visualizer::treemap::render_ascii_treemap;

pub fn render_text_report(report: &BloatAnalysisReport, opts: &Opts) -> String {
    let mut out = String::new();

    out.push_str("================================================================================\n");
    out.push_str(&format!(
        " CARGO-LIGHT DEPENDENCY BLOAT REPORT: {}\n",
        report.workspace_name.to_uppercase()
    ));
    out.push_str("================================================================================\n\n");

    out.push_str(&format!("  Workspace / Package : {}\n", report.root_package));
    out.push_str(&format!("  Total Dependencies  : {}\n", report.total_dependencies));
    out.push_str(&format!("  Direct Dependencies : {}\n", report.direct_dependencies));
    out.push_str(&format!("  Total Size Footprint: {}\n", report.formatted_total_size()));
    out.push_str(&format!("  Total Code Lines    : {} LOC\n", report.total_code_lines));
    out.push_str(&format!("  Avg Macro Weight    : {:.1} / 100.0\n", report.avg_macro_weight));
    out.push_str(&format!("  Avg Monomorph Cost  : {:.1} / 100.0\n", report.avg_monomorph_cost));
    out.push('\n');

    // Prepare top items for ASCII Treemap
    let limit = opts.top.unwrap_or(12);
    let top_crates: Vec<_> = report
        .crate_metrics
        .iter()
        .take(limit)
        .map(|c| {
            let val = match opts.sort_by {
                SortMetric::Size => c.size_bytes as f64,
                SortMetric::Macro => c.derive_macro_weight,
                SortMetric::Monomorph => c.monomorphization_cost,
                SortMetric::Total => c.bloat_score,
            };
            let formatted = match opts.sort_by {
                SortMetric::Size => c.formatted_size(),
                SortMetric::Macro => format!("{:.1} wgt", c.derive_macro_weight),
                SortMetric::Monomorph => format!("{:.1} cost", c.monomorphization_cost),
                SortMetric::Total => format!("{:.1} score", c.bloat_score),
            };
            (c.name.clone(), val, formatted)
        })
        .collect();

    out.push_str(&render_ascii_treemap(&top_crates, 78));
    out.push('\n');

    // Dependency breakdown table
    out.push_str("── Top Dependency Bloat Breakdown ──────────────────────────────────────────────\n");
    out.push_str(&format!(
        "{:<20} {:<10} {:<10} {:<12} {:<12} {:<8}\n",
        "CRATE", "VERSION", "SIZE", "MACRO WGT", "MONOMORPH", "TYPE"
    ));
    out.push_str("────────────────────────────────────────────────────────────────────────────────\n");

    let display_list: Vec<_> = report
        .crate_metrics
        .iter()
        .take(opts.top.unwrap_or(20))
        .collect();

    for c in display_list {
        let dep_type = if c.is_workspace_member {
            "Workspace"
        } else if c.is_direct_dep {
            "Direct"
        } else {
            "Transitive"
        };

        out.push_str(&format!(
            "{:<20} {:<10} {:<10} {:<12.1} {:<12.1} {:<8}\n",
            if c.name.len() > 19 { &c.name[..19] } else { &c.name },
            if c.version.len() > 9 { &c.version[..9] } else { &c.version },
            c.formatted_size(),
            c.derive_macro_weight,
            c.monomorphization_cost,
            dep_type
        ));
    }
    out.push_str("────────────────────────────────────────────────────────────────────────────────\n\n");

    // Duplicate dependencies section
    if !report.duplicates.is_empty() {
        out.push_str("── Duplicate Dependency Versions Detected ──────────────────────────────────────\n");
        for dup in &report.duplicates {
            out.push_str(&format!(
                "  ⚠ {:<18} Versions: {:<25} Wasted: {}\n",
                dup.name,
                dup.versions.join(", "),
                crate::model::format_bytes(dup.wasted_bytes)
            ));
        }
        out.push('\n');
    }

    // Prune Suggestions
    if opts.prune_suggestions || !report.prune_suggestions.is_empty() {
        out.push_str("── Actionable Prune & Replacement Suggestions ──────────────────────────────────\n");
        let suggestions = if let Some(top) = opts.top {
            &report.prune_suggestions[..report.prune_suggestions.len().min(top)]
        } else {
            &report.prune_suggestions[..]
        };

        if suggestions.is_empty() {
            out.push_str("  No critical bloat prune suggestions found. Great job!\n");
        } else {
            for (idx, s) in suggestions.iter().enumerate() {
                out.push_str(&format!(
                    " {:2}. [{}] {}\n     Crate: {} | Potential Savings: {}\n     Advice: {}\n",
                    idx + 1,
                    s.category,
                    s.title,
                    s.crate_name,
                    crate::model::format_bytes(s.potential_savings_bytes),
                    s.description
                ));
                if let Some(ref repl) = s.replacement {
                    out.push_str(&format!("     Suggested Action/Replacement: {}\n", repl));
                }
                out.push('\n');
            }
        }
    }

    out
}
