use anyhow::Result;
use cargo_light::analyzer::Analyzer;
use cargo_light::opts::Opts;
use cargo_light::visualizer::render_report;
use clap::Parser;

fn main() -> Result<()> {
    let opts = Opts::parse();

    let analyzer = Analyzer::new(opts.clone());
    let report = analyzer.analyze()?;

    render_report(report, &opts)?;

    Ok(())
}
