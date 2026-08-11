use std::path::PathBuf;

#[test]
fn test_format_bytes_utility() {
    use cargo_light::model::format_bytes;
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1500), "1.46 KB");
    assert_eq!(format_bytes(2_500_000), "2.38 MB");
    assert_eq!(format_bytes(5_000_000_000), "4.66 GB");
}

#[test]
fn test_treemap_layout_computation() {
    use cargo_light::visualizer::treemap::{compute_treemap_layout, Rect};

    let items = vec![
        ("serde".to_string(), 500.0, "500 KB".to_string()),
        ("tokio".to_string(), 300.0, "300 KB".to_string()),
        ("syn".to_string(), 200.0, "200 KB".to_string()),
    ];

    let rect = Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 50,
    };

    let layout = compute_treemap_layout(&items, rect);
    assert_eq!(layout.len(), 3);
    assert_eq!(layout[0].name, "serde");
    assert!(layout[0].rect.width > 0);
    assert!(layout[0].rect.height > 0);
}

#[test]
fn test_prune_suggestions_and_duplicates() {
    use cargo_light::analyzer::prune::{find_duplicates, generate_prune_suggestions};
    use cargo_light::model::CrateMetrics;

    let crates = vec![
        CrateMetrics {
            name: "syn".to_string(),
            version: "1.0.109".to_string(),
            package_id: "syn 1.0.109".to_string(),
            manifest_path: PathBuf::from("Cargo.toml"),
            source_path: None,
            size_bytes: 400_000,
            code_lines: 5000,
            derive_macro_weight: 50.0,
            derive_count: 10,
            is_proc_macro: false,
            monomorphization_cost: 20.0,
            generic_fn_count: 5,
            is_direct_dep: false,
            is_workspace_member: false,
            dependencies: vec![],
            dependents: vec![],
            depth: 2,
            features_enabled: vec![],
            bloat_score: 45.0,
        },
        CrateMetrics {
            name: "syn".to_string(),
            version: "2.0.48".to_string(),
            package_id: "syn 2.0.48".to_string(),
            manifest_path: PathBuf::from("Cargo.toml"),
            source_path: None,
            size_bytes: 600_000,
            code_lines: 7000,
            derive_macro_weight: 60.0,
            derive_count: 12,
            is_proc_macro: false,
            monomorphization_cost: 25.0,
            generic_fn_count: 6,
            is_direct_dep: false,
            is_workspace_member: false,
            dependencies: vec![],
            dependents: vec![],
            depth: 2,
            features_enabled: vec![],
            bloat_score: 55.0,
        },
    ];

    let dups = find_duplicates(&crates);
    assert_eq!(dups.len(), 1);
    assert_eq!(dups[0].name, "syn");
    assert_eq!(dups[0].versions.len(), 2);

    let suggestions = generate_prune_suggestions(&crates, &dups);
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.crate_name == "syn"));
}
