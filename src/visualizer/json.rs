use crate::model::BloatAnalysisReport;
use crate::opts::Opts;
use anyhow::Result;

pub fn render_json_report(report: &BloatAnalysisReport, opts: &Opts) -> Result<String> {
    if let Some(top) = opts.top {
        let mut truncated = report.clone();
        truncated.crate_metrics.truncate(top);
        truncated.prune_suggestions.truncate(top);
        Ok(serde_json::to_string_pretty(&truncated)?)
    } else {
        Ok(serde_json::to_string_pretty(report)?)
    }
}
