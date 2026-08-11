use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect as RRect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
    Terminal,
};
use std::io::stdout;

use crate::model::{BloatAnalysisReport, CrateMetrics};
use crate::opts::SortMetric;

pub fn run_tui(mut report: BloatAnalysisReport, initial_sort: SortMetric) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut current_sort = initial_sort;
    let mut selected_idx = 0;
    let mut filter_mode = false;
    let mut filter_text = String::new();
    let mut show_prune_only = false;

    let res = main_loop(
        &mut terminal,
        &mut report,
        &mut current_sort,
        &mut selected_idx,
        &mut filter_mode,
        &mut filter_text,
        &mut show_prune_only,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    report: &mut BloatAnalysisReport,
    current_sort: &mut SortMetric,
    selected_idx: &mut usize,
    filter_mode: &mut bool,
    filter_text: &mut String,
    show_prune_only: &mut bool,
) -> Result<()> {
    loop {
        // Sort crate metrics based on current metric
        match current_sort {
            SortMetric::Size => report.crate_metrics.sort_by_key(|b| std::cmp::Reverse(b.size_bytes)),
            SortMetric::Macro => report.crate_metrics.sort_by(|a, b| b.derive_macro_weight.partial_cmp(&a.derive_macro_weight).unwrap_or(std::cmp::Ordering::Equal)),
            SortMetric::Monomorph => report.crate_metrics.sort_by(|a, b| b.monomorphization_cost.partial_cmp(&a.monomorphization_cost).unwrap_or(std::cmp::Ordering::Equal)),
            SortMetric::Total => report.crate_metrics.sort_by(|a, b| b.bloat_score.partial_cmp(&a.bloat_score).unwrap_or(std::cmp::Ordering::Equal)),
        }

        // Filter list
        let filtered_crates: Vec<&CrateMetrics> = report
            .crate_metrics
            .iter()
            .filter(|c| {
                if *show_prune_only {
                    report.prune_suggestions.iter().any(|s| s.crate_name == c.name)
                } else {
                    true
                }
            })
            .filter(|c| {
                if filter_text.is_empty() {
                    true
                } else {
                    c.name.to_lowercase().contains(&filter_text.to_lowercase())
                }
            })
            .collect();

        let count = filtered_crates.len();
        if count > 0 && *selected_idx >= count {
            *selected_idx = count - 1;
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Header
                    Constraint::Min(10),   // Main Dual-Pane
                    Constraint::Length(3), // Footer / Filter bar
                ])
                .split(f.area());

            // Header
            let header_text = vec![
                Line::from(vec![
                    Span::styled("CARGO-LIGHT ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled("⚡ Rust Dependency Bloat & Macro Analyzer", Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::raw("Workspace: "),
                    Span::styled(&report.workspace_name, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw(" | Total Size: "),
                    Span::styled(report.formatted_total_size(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    Span::raw(" | Total Deps: "),
                    Span::styled(format!("{}", report.total_dependencies), Style::default().fg(Color::White)),
                    Span::raw(" | Avg Macro: "),
                    Span::styled(format!("{:.1}", report.avg_macro_weight), Style::default().fg(Color::Yellow)),
                    Span::raw(" | Avg Monomorph: "),
                    Span::styled(format!("{:.1}", report.avg_monomorph_cost), Style::default().fg(Color::Blue)),
                ]),
            ];

            let header = Paragraph::new(header_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Global Workspace Summary ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(header, chunks[0]);

            // Main dual pane
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(chunks[1]);

            // Left Pane: List of crates with size/macro bars
            let items: Vec<ListItem> = filtered_crates
                .iter()
                .enumerate()
                .map(|(idx, c)| {
                    let is_selected = idx == *selected_idx;

                    let (metric_str, color) = match current_sort {
                        SortMetric::Size => (c.formatted_size(), Color::Magenta),
                        SortMetric::Macro => (format!("{:.1} wgt", c.derive_macro_weight), Color::Yellow),
                        SortMetric::Monomorph => (format!("{:.1} cost", c.monomorphization_cost), Color::Blue),
                        SortMetric::Total => (format!("{:.1} bloat", c.bloat_score), Color::Red),
                    };

                    let flag = if c.is_direct_dep { "[Direct]" } else { "[Trans]" };

                    let style = if is_selected {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    let line = Line::from(vec![
                        Span::styled(format!("{:<20}", c.name), style),
                        Span::styled(format!("{:<10}", c.version), style),
                        Span::styled(format!("{:<12}", metric_str), Style::default().fg(if is_selected { Color::Black } else { color })),
                        Span::styled(flag.to_string(), Style::default().fg(if is_selected { Color::Black } else { Color::DarkGray })),
                    ]);

                    ListItem::new(line)
                })
                .collect();

            let mut state = ListState::default();
            if count > 0 {
                state.select(Some(*selected_idx));
            }

            let title_sort = format!(" Dependencies (Sorted by: {}) ", current_sort);
            let list_widget = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title_sort)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black));

            f.render_stateful_widget(list_widget, main_chunks[0], &mut state);

            // Right Pane: Inspector Details Card
            if let Some(selected_crate) = filtered_crates.get(*selected_idx) {
                render_inspector_card(f, main_chunks[1], selected_crate, report);
            } else {
                let empty = Paragraph::new("No crates match filter criteria.").block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Crate Inspector "),
                );
                f.render_widget(empty, main_chunks[1]);
            }

            // Footer / Search Bar
            let footer_content = if *filter_mode {
                Line::from(vec![
                    Span::styled(" SEARCH FILTER: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(filter_text.as_str()),
                    Span::styled("█", Style::default().fg(Color::Cyan)),
                    Span::styled(" (Press Enter to apply, Esc to clear)", Style::default().fg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("[q] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Quit | "),
                    Span::styled("[↑/↓/j/k] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Navigate | "),
                    Span::styled("[m] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw("Sort Metric | "),
                    Span::styled("[p] ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                    Span::raw(if *show_prune_only { "Show All | " } else { "Prune Only | " }),
                    Span::styled("[/] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Search Filter"),
                ])
            };

            let footer = Paragraph::new(footer_content).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle terminal key events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if *filter_mode {
                    match key.code {
                        KeyCode::Enter | KeyCode::Esc => {
                            *filter_mode = false;
                        }
                        KeyCode::Backspace => {
                            filter_text.pop();
                        }
                        KeyCode::Char(c) => {
                            filter_text.push(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                        KeyCode::Down | KeyCode::Char('j') => {
                            if count > 0 && *selected_idx + 1 < count {
                                *selected_idx += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if *selected_idx > 0 {
                                *selected_idx -= 1;
                            }
                        }
                        KeyCode::Char('m') => {
                            *current_sort = match current_sort {
                                SortMetric::Size => SortMetric::Macro,
                                SortMetric::Macro => SortMetric::Monomorph,
                                SortMetric::Monomorph => SortMetric::Total,
                                SortMetric::Total => SortMetric::Size,
                            };
                        }
                        KeyCode::Char('p') => {
                            *show_prune_only = !*show_prune_only;
                            *selected_idx = 0;
                        }
                        KeyCode::Char('/') => {
                            *filter_mode = true;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

fn render_inspector_card(
    f: &mut ratatui::Frame,
    area: RRect,
    c: &CrateMetrics,
    report: &BloatAnalysisReport,
) {
    let card_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Overview
            Constraint::Length(3),  // Macro Gauge
            Constraint::Length(3),  // Monomorph Gauge
            Constraint::Min(6),     // Recommendations / Advice
        ])
        .split(area);

    // Overview block
    let overview_lines = vec![
        Line::from(vec![
            Span::styled("Crate: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&c.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" (v"),
            Span::styled(&c.version, Style::default().fg(Color::White)),
            Span::raw(")"),
        ]),
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if c.is_proc_macro { "Proc-Macro Crate" } else if c.is_direct_dep { "Direct Dependency" } else { "Transitive Dependency" },
                Style::default().fg(Color::Green),
            ),
            Span::raw(" | Depth: "),
            Span::styled(format!("{}", c.depth), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("Disk Size: ", Style::default().fg(Color::DarkGray)),
            Span::styled(c.formatted_size(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw(" | Code Lines: "),
            Span::styled(format!("{} LOC", c.code_lines), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Derive Count: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", c.derive_count), Style::default().fg(Color::Yellow)),
            Span::raw(" | Generic Items: "),
            Span::styled(format!("{}", c.generic_fn_count), Style::default().fg(Color::Blue)),
        ]),
        Line::from(vec![
            Span::styled("Enabled Features: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if c.features_enabled.is_empty() { "default".to_string() } else { c.features_enabled.join(", ") },
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let overview = Paragraph::new(overview_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Inspector: {} ", c.name))
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(overview, card_chunks[0]);

    // Derive Macro Gauge
    let macro_gauge = Gauge::default()
        .block(Block::default().title(" Derive Macro Weight Score ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .percent(c.derive_macro_weight.clamp(0.0, 100.0) as u16)
        .label(format!("{:.1} / 100.0", c.derive_macro_weight));
    f.render_widget(macro_gauge, card_chunks[1]);

    // Monomorphization Gauge
    let mono_gauge = Gauge::default()
        .block(Block::default().title(" Monomorphization Cost Score ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black))
        .percent(c.monomorphization_cost.clamp(0.0, 100.0) as u16)
        .label(format!("{:.1} / 100.0", c.monomorphization_cost));
    f.render_widget(mono_gauge, card_chunks[2]);

    // Prune suggestions & Advice
    let mut advice_lines = Vec::new();

    let matching_suggestions: Vec<_> = report
        .prune_suggestions
        .iter()
        .filter(|s| s.crate_name == c.name)
        .collect();

    if !matching_suggestions.is_empty() {
        advice_lines.push(Line::from(Span::styled(
            "⚡ Actionable Bloat Optimization:",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        for s in matching_suggestions {
            advice_lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Red)),
                Span::styled(&s.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            advice_lines.push(Line::from(Span::styled(
                format!("  Savings: {}", crate::model::format_bytes(s.potential_savings_bytes)),
                Style::default().fg(Color::Green),
            )));
            advice_lines.push(Line::from(Span::styled(
                format!("  Detail: {}", s.description),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        advice_lines.push(Line::from(Span::styled(
            "✔ No critical bloat or replacement warning for this crate.",
            Style::default().fg(Color::Green),
        )));
    }

    if let Some(dup) = report.duplicates.iter().find(|d| d.name == c.name) {
        advice_lines.push(Line::from(""));
        advice_lines.push(Line::from(Span::styled(
            format!("⚠ DUPLICATE VERSIONS IN LOCKFILE: {}", dup.versions.join(", ")),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
    }

    let advice_box = Paragraph::new(advice_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Prune & Optimization Advice ")
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(advice_box, card_chunks[3]);
}
