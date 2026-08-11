# cargo-light ⚡

> Production-quality Rust binary dependency bloat analyzer with interactive Ratatui Treemap visualizer, derive macro expansion weight estimation, generic monomorphization cost analysis, and automated prune suggestions.

---

## 🚀 Key Features

- **Dependency Bloat Treemap**: Visualize disk footprint, LOC, and code weight across all workspace dependencies using interactive Ratatui TUI or human-readable ASCII treemaps.
- **Derive Macro Expansion Weight**: Scores proc-macro overhead (e.g. `syn`, `quote`, `proc-macro2`, `serde_derive`) to identify compile-time bloat drivers.
- **Monomorphization Cost Analysis**: Measures generic function instantiations and generic item density `<T>` to pinpoint runtime code multiplication bloat.
- **Actionable Prune Suggestions**: Automatically detects duplicate crate versions in `Cargo.lock` and recommends lighter, modern crate alternatives (e.g. `reqwest` → `ureq`, `serde_json` → `miniserde`, `thiserror` → manual `std::fmt::Display`).
- **Flexible Output Formats**: Supports interactive TUI (`tui`), stdout text report (`text`), and structured machine-readable JSON (`json`).

---

## 📦 Installation & Usage

```bash
# Clone and build locally
git clone https://github.com/your-user/cargo-light.git
cd cargo-light
cargo install --path .

# Run inside any Rust project directory
cargo-light
```

### CLI Command Options

```text
Usage: cargo-light [OPTIONS]

Options:
  -m, --manifest-path <PATH>  Path to Cargo.toml manifest file [default: ./Cargo.toml]
  -t, --top <N>               Limit the number of top dependencies displayed in text/json mode
  -f, --format <FORMAT>       Output format: tui, text, or json [default: tui if TTY, else text]
  -p, --prune-suggestions     Display actionable prune and replacement suggestions for bloat reduction
  -s, --sort-by <SORT_BY>     Primary metric to sort dependencies by (size, macro, monomorph, total) [default: size]
  -w, --workspace             Analyze all workspace packages recursively
      --offline               Offline mode: avoid cargo network updates and rely on local metadata/cache
  -F, --filter <FILTER>       Filter dependencies by name substring
  -h, --help                  Print help
  -V, --version               Print version
```

---

## 🎨 ASCII Treemap & Report Example

When executed with `--format text --prune-suggestions`, `cargo-light` outputs an ASCII treemap breakdown and prune recommendations:

```text
================================================================================
 CARGO-LIGHT DEPENDENCY BLOAT REPORT: CARGO-LIGHT
================================================================================

  Workspace / Package : cargo-light
  Total Dependencies  : 114
  Direct Dependencies : 12
  Total Size Footprint: 1.29 GB
  Total Code Lines    : 2,574,722 LOC
  Avg Macro Weight    : 49.1 / 100.0
  Avg Monomorph Cost  : 74.4 / 100.0

┌─ Dependency Size & Weight Footprint Treemap ──────────────────────────────────┐
│ cargo-light        [██████████████████████████████████████████████░░░░░░░]    1.29 GB (87.3%) │
│ winapi-x86_64-pc-windows-gnu [██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   50.70 MB ( 3.4%) │
│ winapi-i686-pc-windows-gnu [██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   48.57 MB ( 3.2%) │
│ windows-sys        [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   17.34 MB ( 1.1%) │
│ windows-sys        [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   16.36 MB ( 1.1%) │
│ linux-raw-sys      [█░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   16.15 MB ( 1.1%) │
└───────────────────────────────────────────────────────────────────────────────┘

── Top Dependency Bloat Breakdown ──────────────────────────────────────────────
CRATE                VERSION    SIZE       MACRO WGT    MONOMORPH    TYPE    
────────────────────────────────────────────────────────────────────────────────
cargo-light          0.1.0      1.29 GB    57.0         100.0        Workspace
winapi-x86_64-pc-wi  0.4.0      50.70 MB   0.0          100.0        Transitive
windows-sys          0.61.2     17.34 MB   100.0        90.9         Transitive
windows-sys          0.59.0     16.36 MB   100.0        85.8         Transitive
linux-raw-sys        0.12.1     16.15 MB   100.0        100.0        Transitive
────────────────────────────────────────────────────────────────────────────────

── Duplicate Dependency Versions Detected ──────────────────────────────────────
  ⚠ windows-sys        Versions: 0.59.0, 0.61.2            Wasted: 17.34 MB
  ⚠ linux-raw-sys      Versions: 0.12.1, 0.4.15            Wasted: 16.15 MB
  ⚠ syn                Versions: 2.0.119, 3.0.3            Wasted: 2.25 MB

── Actionable Prune & Replacement Suggestions ──────────────────────────────────
 1. [Duplicate Version] Consolidate duplicate versions of 'windows-sys'
     Crate: windows-sys | Potential Savings: 17.34 MB
     Advice: Multiple versions detected: 0.59.0, 0.61.2. Use 'cargo tree -d' and update Cargo.toml or run 'cargo update -p windows-sys' to consolidate.
     Suggested Action/Replacement: Unify onto version 0.61.2

 2. [Lighter Alternative Available] Consider 'miniserde' or manual parsing for light JSON
     Crate: serde_json | Potential Savings: 1.10 MB
     Advice: 'serde_json' has generic monomorphization overhead. For small CLI utilities, 'miniserde' or custom deserialization reduces code size.
     Suggested Action/Replacement: miniserde
```

---

## 🖥️ Interactive TUI Controls

In interactive mode (`--format tui`):

| Key Binding | Action |
|:---|:---|
| `q` / `Esc` | Exit `cargo-light` TUI |
| `↑` / `↓` / `k` / `j` | Navigate dependency list |
| `m` | Cycle sort metric (`Size` ➔ `Macro Weight` ➔ `Monomorph Cost` ➔ `Total Bloat`) |
| `p` | Toggle Prune-Only view (filter crates with optimization warnings) |
| `/` | Open live search filter bar |

---

## 💡 Rust Binary Bloat Optimization Guide

### 1. Eliminating Duplicate Dependency Versions
When different transitive dependencies depend on different major or minor versions of the same crate (e.g. `syn v1` and `syn v2`), Cargo compiles both into your final artifact, wasting disk space and memory.
- **Diagnosis**: Run `cargo tree -d` or use `cargo-light -p`.
- **Fix**: Update top-level Cargo dependencies to align versions or execute `cargo update -p <crate>`.

### 2. Reducing Derive Macro Overhead
Proc-macros (`#[derive(Serialize, Deserialize)]`, `#[derive(Parser)]`) force Cargo to pull in compiler infrastructure crates like `syn`, `quote`, `proc-macro2`.
- **Optimization**: For small error enums or CLI structs, implement `std::fmt::Display` and `std::error::Error` manually instead of using proc-macro derives.

### 3. Mitigating Monomorphization Code Multiplication
Generics in Rust generate distinct compiled machine code for every type parameter instantiated (`fn process<T>(...)`). Heavy generic traits across large data structures cause binary size inflation.
- **Optimization**: Use dynamic dispatch via trait objects (`&dyn Trait` or `Box<dyn Trait>`) for non-performance-critical paths to reduce monomorphic function duplicates.

### 4. Lightweight Alternative Crates
| Heavy Crate | Lightweight Alternative | Saved Footprint | Use Case |
|:---|:---|:---|:---|
| `reqwest` | `ureq` / `minireq` | ~2.5 MB | Synchronous HTTP requests without Tokio/Hyper |
| `serde_json` | `miniserde` | ~1.0 MB | Simple JSON deserialization |
| `thiserror` | Manual `Display` impl | ~400 KB | Small library error enums |
| `regex` | `regex-lite` / `memchr` | ~1.2 MB | Basic string scanning & pattern matching |

---

## 🧪 Testing & Verification

```bash
# Run unit & integration test suite
cargo test

# Enforce strict lints with zero warnings
cargo clippy -- -D warnings
```

---

## 📜 License

Licensed under Apache-2.0 or MIT at your option.
