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
    #[allow(dead_code)]
    #[serde(default)]
    exceeds_200k_tokens: Option<bool>,
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
    #[serde(default)]
    total_lines_added: Option<u64>,
    #[serde(default)]
    total_lines_removed: Option<u64>,
}

#[derive(Deserialize)]
struct ContextWindow {
    #[serde(default)]
    used_percentage: Option<f64>,
    #[serde(default)]
    context_window_size: Option<u64>,
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

fn render_drift(workspace: Option<&Workspace>) -> Option<String> {
    let ws = workspace?;
    let project_dir = ws.project_dir.as_deref()?;
    let current_dir = ws.current_dir.as_str();

    if current_dir == project_dir {
        return None;
    }

    let project_name = dir_name(project_dir);
    Some(format!(" {YELLOW}⚠️ ↩ {project_name}{RESET}"))
}

fn render_line1(input: &StatusInput, git: Option<&GitInfo>) -> String {
    let model = &input.model.display_name;
    let dir = input
        .workspace
        .as_ref()
        .map(|w| dir_name(&w.current_dir))
        .unwrap_or("?");

    let mut line = format!("{CYAN}[{model}]{RESET} 📁 {dir}");

    if let Some(drift) = render_drift(input.workspace.as_ref()) {
        line.push_str(&drift);
    }

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

fn format_context_size(size: Option<u64>) -> &'static str {
    match size {
        Some(s) if s >= 1_000_000 => "1M",
        Some(s) if s >= 200_000 => "200k",
        Some(_) => "?k",
        None => "",
    }
}

fn compact_nudge(pct: u8) -> &'static str {
    if pct >= 70 && pct < 85 {
        " ⚡ compact soon"
    } else {
        ""
    }
}

fn render_line2(input: &StatusInput) -> String {
    let ctx = input.context_window.as_ref();
    let pct = ctx.and_then(|c| c.used_percentage).unwrap_or(0.0) as u8;

    let bar = render_context_bar(pct);

    // Context window size indicator
    let size_label = format_context_size(ctx.and_then(|c| c.context_window_size));
    let size_str = if size_label.is_empty() {
        String::new()
    } else {
        format!(" ({size_label})")
    };

    // Compaction nudge in the 70-84% zone
    let nudge = compact_nudge(pct);
    let nudge_str = if nudge.is_empty() {
        String::new()
    } else {
        format!("{YELLOW}{nudge}{RESET}")
    };

    let cost_data = input.cost.as_ref();
    let cost = cost_data.and_then(|c| c.total_cost_usd).unwrap_or(0.0);

    let duration_ms = cost_data.and_then(|c| c.total_duration_ms).unwrap_or(0);
    let mins = duration_ms / 60_000;
    let secs = (duration_ms % 60_000) / 1_000;

    let mut line =
        format!("{bar}{size_str}{nudge_str} | {YELLOW}${cost:.2}{RESET} | ⏱️ {mins}m {secs}s");

    // Lines added/removed
    let added = cost_data.and_then(|c| c.total_lines_added).unwrap_or(0);
    let removed = cost_data.and_then(|c| c.total_lines_removed).unwrap_or(0);
    if added > 0 || removed > 0 {
        line.push_str(&format!(" | {GREEN}+{added}{RESET} {RED}-{removed}{RESET}"));

        // Lines per dollar (only when cost is meaningful)
        if cost > 0.001 {
            let loc_per_dollar = ((added + removed) as f64 / cost).round() as u64;
            line.push_str(&format!(" {CYAN}({loc_per_dollar} loc/$){RESET}"));
        }
    }

    line
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
