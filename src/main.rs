use serde::Deserialize;
use std::io::Read as _;
use std::path::Path;

// ── ANSI colors ──────────────────────────────────────────────────────
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

// ── JSON input types (Claude Code status line protocol) ──────────────

#[derive(Deserialize)]
struct StatusInput {
    model: Model,
    #[serde(default)]
    workspace: Option<Workspace>,
    #[serde(default)]
    cost: Option<Cost>,
    #[serde(default)]
    context_window: Option<ContextWindow>,
}

#[derive(Deserialize)]
struct Model {
    display_name: String,
}

#[derive(Deserialize)]
struct Workspace {
    current_dir: String,
    #[allow(dead_code)]
    project_dir: Option<String>,
}

#[derive(Deserialize)]
struct Cost {
    #[serde(default)]
    total_cost_usd: Option<f64>,
    #[serde(default)]
    total_duration_ms: Option<u64>,
}

#[derive(Deserialize)]
struct ContextWindow {
    #[serde(default)]
    used_percentage: Option<f64>,
}

// ── Git info via gix (pure Rust, no OpenSSL) ─────────────────────────

struct GitInfo {
    branch: String,
    staged: u32,
    modified: u32,
}

fn git_info(cwd: &str) -> Option<GitInfo> {
    let repo = gix::discover(cwd).ok()?;

    // Branch name (or short hash if detached)
    let branch = match repo.head_name().ok()? {
        Some(name) => name.shorten().to_string(),
        None => {
            let id = repo.head_id().ok()?;
            format!("{:.7}", id)
        }
    };

    // Staged + modified counts via gix status
    let mut staged = 0u32;
    let mut modified = 0u32;

    let status_iter = repo
        .status(gix::progress::Discard)
        .ok()?
        .into_iter(std::iter::empty::<gix::bstr::BString>())
        .ok()?;

    for item in status_iter {
        let Ok(item) = item else { continue };
        match item {
            gix::status::Item::TreeIndex(_) => staged += 1,
            gix::status::Item::IndexWorktree(change) => {
                use gix::status::index_worktree::Item as IW;
                match change {
                    IW::Modification { .. } | IW::Rewrite { .. } => modified += 1,
                    _ => {}
                }
            }
        }
    }

    Some(GitInfo {
        branch,
        staged,
        modified,
    })
}

// ── Rendering ────────────────────────────────────────────────────────

fn dir_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

fn render_line1(input: &StatusInput, git: Option<&GitInfo>) -> String {
    let model = &input.model.display_name;
    let dir = input
        .workspace
        .as_ref()
        .map(|w| dir_name(&w.current_dir))
        .unwrap_or("?");

    let mut line = format!("{CYAN}[{model}]{RESET} 📁 {dir}");

    if let Some(g) = git {
        line.push_str(&format!(" | 🌿 {}", g.branch));
        if g.staged > 0 {
            line.push_str(&format!(" {GREEN}+{}{RESET}", g.staged));
        }
        if g.modified > 0 {
            line.push_str(&format!(" {YELLOW}~{}{RESET}", g.modified));
        }
    }

    line
}

fn render_context_bar(pct: u8) -> String {
    let color = if pct >= 90 {
        RED
    } else if pct >= 70 {
        YELLOW
    } else {
        GREEN
    };

    let filled = (pct as usize) / 10;
    let empty = 10 - filled;
    let bar: String = "█".repeat(filled) + &"░".repeat(empty);

    format!("{color}{bar}{RESET} {pct}%")
}

fn render_line2(input: &StatusInput) -> String {
    let pct = input
        .context_window
        .as_ref()
        .and_then(|c| c.used_percentage)
        .unwrap_or(0.0) as u8;

    let bar = render_context_bar(pct);

    let cost = input
        .cost
        .as_ref()
        .and_then(|c| c.total_cost_usd)
        .unwrap_or(0.0);

    let duration_ms = input
        .cost
        .as_ref()
        .and_then(|c| c.total_duration_ms)
        .unwrap_or(0);
    let mins = duration_ms / 60_000;
    let secs = (duration_ms % 60_000) / 1_000;

    format!("{bar} | {YELLOW}${cost:.2}{RESET} | ⏱️ {mins}m {secs}s")
}

// ── Main ─────────────────────────────────────────────────────────────

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;

    let input: StatusInput = serde_json::from_str(&buf)?;

    let cwd = input
        .workspace
        .as_ref()
        .map(|w| w.current_dir.as_str())
        .unwrap_or(".");

    let git = git_info(cwd);

    println!("{}", render_line1(&input, git.as_ref()));
    println!("{}", render_line2(&input));

    Ok(())
}

fn main() {
    if let Err(_) = run() {
        // Never exit non-zero — that blanks the status bar.
        // Print a safe fallback instead.
        println!("[starline]");
    }
}
