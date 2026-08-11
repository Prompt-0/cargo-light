use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct SourceAnalysisResult {
    pub size_bytes: u64,
    pub code_lines: u64,
    pub derive_count: usize,
    pub generic_fn_count: usize,
    pub is_proc_macro: bool,
    pub macro_rules_count: usize,
}

pub fn analyze_source_dir(dir_path: &Path, is_proc_macro_crate: bool) -> SourceAnalysisResult {
    let mut total_bytes: u64 = 0;
    let mut total_lines: u64 = 0;
    let mut derive_count: usize = 0;
    let mut generic_fn_count: usize = 0;
    let mut macro_rules_count: usize = 0;

    if !dir_path.exists() {
        return SourceAnalysisResult {
            size_bytes: 1024, // Fallback non-zero estimation
            code_lines: 50,
            derive_count: 0,
            generic_fn_count: 0,
            is_proc_macro: is_proc_macro_crate,
            macro_rules_count: 0,
        };
    }

    for entry in WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_bytes += metadata.len();
            }

            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(path) {
                    let lines = content.lines().count() as u64;
                    total_lines += lines;

                    // Analyze rust source patterns
                    for line in content.lines() {
                        let trimmed = line.trim();
                        
                        // Count derives
                        if trimmed.contains("#[derive(") || trimmed.contains("derive(") {
                            // count items inside derive if possible or count occurrences
                            let count = trimmed.matches("derive(").count();
                            derive_count += count.max(1);
                        }

                        // Count macro_rules!
                        if trimmed.contains("macro_rules!") {
                            macro_rules_count += 1;
                        }

                        // Count generics signatures e.g., fn foo<T>, struct Bar<T>, impl<T>
                        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") 
                            || trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ")
                            || trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ")
                            || trimmed.starts_with("impl") || trimmed.starts_with("pub trait ") || trimmed.starts_with("trait "))
                            && trimmed.contains('<') && trimmed.contains('>') {
                            generic_fn_count += 1;
                        }
                    }
                }
            }
        }
    }

    SourceAnalysisResult {
        size_bytes: total_bytes,
        code_lines: total_lines,
        derive_count,
        generic_fn_count,
        is_proc_macro: is_proc_macro_crate,
        macro_rules_count,
    }
}

pub fn calculate_derive_macro_weight(
    derive_count: usize,
    is_proc_macro: bool,
    transitive_macro_deps: usize,
    code_lines: u64,
) -> f64 {
    let mut score = 0.0;
    
    if is_proc_macro {
        score += 40.0;
    }

    score += (derive_count as f64) * 3.5;
    score += (transitive_macro_deps as f64) * 12.0;

    if code_lines > 0 {
        let derive_density = (derive_count as f64) / (code_lines as f64);
        score += derive_density * 500.0;
    }

    score.min(100.0)
}

pub fn calculate_monomorphization_cost(
    generic_fn_count: usize,
    code_lines: u64,
    size_bytes: u64,
) -> f64 {
    let mut score = 0.0;

    score += (generic_fn_count as f64) * 2.5;

    if code_lines > 0 {
        let generic_density = (generic_fn_count as f64) / (code_lines as f64);
        score += generic_density * 800.0;
    }

    // Larger generic crates expand significantly when compiled with many instantiations
    if size_bytes > 500_000 {
        score += (size_bytes as f64 / 1_000_000.0) * 5.0;
    }

    score.min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_weights() {
        let macro_w = calculate_derive_macro_weight(10, false, 2, 500);
        assert!(macro_w > 0.0 && macro_w <= 100.0);

        let mono_w = calculate_monomorphization_cost(15, 600, 200_000);
        assert!(mono_w > 0.0 && mono_w <= 100.0);
    }
}
