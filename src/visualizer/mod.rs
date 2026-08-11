pub mod json;
pub mod text;
pub mod treemap;
pub mod tui;

use crate::model::BloatAnalysisReport;
use crate::opts::{Opts, OutputFormat};
use anyhow::Result;
use std::io::IsTerminal;

pub fn render_report(report: BloatAnalysisReport, opts: &Opts) -> Result<()> {
    // Determine output format
    let format = opts.format.clone().unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            OutputFormat::Tui
        } else {
            OutputFormat::Text
        }
    });

    match format {
        OutputFormat::Tui => {
            if std::io::stdout().is_terminal() {
                tui::run_tui(report, opts.sort_by)?;
            } else {
                // Fallback to text mode if not attached to interactive TTY
                let text = text::render_text_report(&report, opts);
                print!("{}", text);
            }
        }
        OutputFormat::Text => {
            let text = text::render_text_report(&report, opts);
            print!("{}", text);
        }
        OutputFormat::Json => {
            let json_str = json::render_json_report(&report, opts)?;
            println!("{}", json_str);
        }
    }

    Ok(())
}
