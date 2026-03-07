use serde::Deserialize;
use std::io::{IsTerminal, Read as _};
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
    // Kept for forward-compatibility with the Claude Code JSON protocol.
    // The field is present in the input schema but not yet used by starline.
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
    // Used in render_drift to detect directory drift from project root.
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

// ── Git info via subprocess ──────────────────────────────────────────

struct GitInfo {
    branch: String,
    staged: u32,
    modified: u32,
    conflicts: u32,
}

fn git_info(cwd: &str) -> Option<GitInfo> {
    use std::process::Command;

    let branch_out = Command::new("git")
        .args(["--no-optional-locks", "rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !branch_out.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&branch_out.stdout).trim().to_string();

    // Detached HEAD returns "HEAD" — fall back to short hash
    let branch = if branch == "HEAD" {
        let hash_out = Command::new("git")
            .args(["--no-optional-locks", "rev-parse", "--short", "HEAD"])
            .current_dir(cwd)
            .output()
            .ok()?;
        String::from_utf8_lossy(&hash_out.stdout).trim().to_string()
    } else {
        branch
    };

    let status_out = Command::new("git")
        .args(["--no-optional-locks", "status", "--porcelain=v1", "-uno"])
        .current_dir(cwd)
        .output()
        .ok()?;

    let (staged, modified, conflicts) =
        parse_porcelain(&String::from_utf8_lossy(&status_out.stdout));

    Some(GitInfo {
        branch,
        staged,
        modified,
        conflicts,
    })
}

fn parse_porcelain(output: &str) -> (u32, u32, u32) {
    let mut staged = 0u32;
    let mut modified = 0u32;
    let mut conflicts = 0u32;

    for line in output.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 2 {
            continue;
        }
        let (x, y) = (bytes[0], bytes[1]);

        // Conflict markers: UU, AA, DD, AU, UA, DU, UD
        if matches!(
            (x, y),
            (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D')
        ) {
            conflicts += 1;
        } else {
            if x != b' ' && x != b'?' {
                staged += 1;
            }
            if y != b' ' && y != b'?' {
                modified += 1;
            }
        }
    }

    (staged, modified, conflicts)
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
        if g.conflicts > 0 {
            line.push_str(&format!(" {RED}!{}{RESET}", g.conflicts));
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

fn format_context_size(size: Option<u64>) -> String {
    match size {
        Some(s) if s >= 1_000_000 => format!("{}M", s / 1_000_000),
        Some(s) if s >= 1_000 => format!("{}k", s / 1_000),
        Some(s) => format!("{}", s),
        None => String::new(),
    }
}

fn compact_nudge(pct: u8) -> &'static str {
    if (70..90).contains(&pct) {
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

// ── Usage ─────────────────────────────────────────────────────────────

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage() {
    eprintln!(
        "starline v{VERSION} — a fast Rust status line for Claude Code\n\
         \n\
         Usage: pipe JSON into starline per\n\
         https://code.claude.com/docs/en/statusline\n\
         \n\
         Install/update:\n  \
         https://github.com/anoldguy/starline/releases"
    );
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

fn wants_help() -> bool {
    std::env::args()
        .skip(1)
        .any(|a| a == "--help" || a == "-h" || a == "help")
}

fn wants_version() -> bool {
    std::env::args()
        .skip(1)
        .any(|a| a == "--version" || a == "-V" || a == "version")
}

fn main() {
    if wants_version() {
        eprintln!("starline v{VERSION}");
        return;
    }

    if wants_help() || std::io::stdin().is_terminal() {
        print_usage();
        return;
    }

    if let Err(e) = run() {
        // Log to stderr — invisible to the Claude Code status bar protocol.
        eprintln!("[starline] error: {e}");
        // Never exit non-zero — that blanks the status bar.
        // Print two lines — Claude Code expects exactly two lines of output.
        println!("[starline]");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── render_context_bar ──────────────────────────────────────────

    #[test]
    fn context_bar_empty() {
        let bar = render_context_bar(0);
        assert!(bar.contains("░░░░░░░░░░"));
        assert!(bar.contains("0%"));
    }

    #[test]
    fn context_bar_partial() {
        let bar = render_context_bar(50);
        assert!(bar.contains("█████░░░░░"));
        assert!(bar.contains("50%"));
    }

    #[test]
    fn context_bar_full() {
        let bar = render_context_bar(100);
        assert!(bar.contains("██████████"));
        assert!(bar.contains("100%"));
    }

    #[test]
    fn context_bar_color_green_below_70() {
        let bar = render_context_bar(69);
        assert!(bar.starts_with(GREEN));
    }

    #[test]
    fn context_bar_color_yellow_at_70() {
        let bar = render_context_bar(70);
        assert!(bar.starts_with(YELLOW));
    }

    #[test]
    fn context_bar_color_red_at_90() {
        let bar = render_context_bar(90);
        assert!(bar.starts_with(RED));
    }

    // ── format_context_size ─────────────────────────────────────────

    #[test]
    fn context_size_none() {
        assert_eq!(format_context_size(None), "");
    }

    #[test]
    fn context_size_sub_1k() {
        assert_eq!(format_context_size(Some(500)), "500");
    }

    #[test]
    fn context_size_thousands() {
        assert_eq!(format_context_size(Some(128_000)), "128k");
    }

    #[test]
    fn context_size_200k() {
        assert_eq!(format_context_size(Some(200_000)), "200k");
    }

    #[test]
    fn context_size_500k() {
        assert_eq!(format_context_size(Some(500_000)), "500k");
    }

    #[test]
    fn context_size_1m() {
        assert_eq!(format_context_size(Some(1_000_000)), "1M");
    }

    #[test]
    fn context_size_2m() {
        assert_eq!(format_context_size(Some(2_000_000)), "2M");
    }

    // ── compact_nudge ───────────────────────────────────────────────

    #[test]
    fn nudge_below_threshold() {
        assert_eq!(compact_nudge(69), "");
    }

    #[test]
    fn nudge_at_70() {
        assert_eq!(compact_nudge(70), " ⚡ compact soon");
    }

    #[test]
    fn nudge_at_84() {
        assert_eq!(compact_nudge(84), " ⚡ compact soon");
    }

    #[test]
    fn nudge_at_89() {
        assert_eq!(compact_nudge(89), " ⚡ compact soon");
    }

    #[test]
    fn nudge_above_90() {
        assert_eq!(compact_nudge(90), "");
    }

    #[test]
    fn nudge_at_100() {
        assert_eq!(compact_nudge(100), "");
    }

    // ── dir_name ────────────────────────────────────────────────────

    #[test]
    fn dir_name_simple() {
        assert_eq!(dir_name("/home/user/project"), "project");
    }

    #[test]
    fn dir_name_nested() {
        assert_eq!(dir_name("/a/b/c/d"), "d");
    }

    #[test]
    fn dir_name_root() {
        // Path::file_name returns None for "/", so we fall back to the input.
        assert_eq!(dir_name("/"), "/");
    }

    #[test]
    fn dir_name_trailing_slash() {
        // Path::file_name handles trailing slashes by ignoring them.
        assert_eq!(dir_name("/home/user/project/"), "project");
    }

    // ── render_drift ────────────────────────────────────────────────

    #[test]
    fn drift_none_when_no_workspace() {
        assert!(render_drift(None).is_none());
    }

    #[test]
    fn drift_none_when_same_dir() {
        let ws = Workspace {
            current_dir: "/home/user/project".to_string(),
            project_dir: Some("/home/user/project".to_string()),
        };
        assert!(render_drift(Some(&ws)).is_none());
    }

    #[test]
    fn drift_none_when_no_project_dir() {
        let ws = Workspace {
            current_dir: "/home/user/project".to_string(),
            project_dir: None,
        };
        assert!(render_drift(Some(&ws)).is_none());
    }

    #[test]
    fn drift_shows_project_name_when_dirs_differ() {
        let ws = Workspace {
            current_dir: "/home/user/project/subdir".to_string(),
            project_dir: Some("/home/user/project".to_string()),
        };
        let drift = render_drift(Some(&ws)).unwrap();
        assert!(drift.contains("project"));
        assert!(drift.contains("↩"));
    }

    // ── parse_porcelain ────────────────────────────────────────────

    #[test]
    fn porcelain_empty() {
        assert_eq!(parse_porcelain(""), (0, 0, 0));
    }

    #[test]
    fn porcelain_staged_files() {
        let output = "A  new_file.rs\nM  changed.rs\n";
        assert_eq!(parse_porcelain(output), (2, 0, 0));
    }

    #[test]
    fn porcelain_modified_files() {
        let output = " M unstaged.rs\n M another.rs\n";
        assert_eq!(parse_porcelain(output), (0, 2, 0));
    }

    #[test]
    fn porcelain_mixed() {
        let output = "M  staged.rs\n M unstaged.rs\nMM both.rs\n";
        assert_eq!(parse_porcelain(output), (2, 2, 0));
    }

    #[test]
    fn porcelain_conflicts() {
        let output = "UU conflicted.rs\nAA both_added.rs\nDD both_deleted.rs\n";
        assert_eq!(parse_porcelain(output), (0, 0, 3));
    }

    #[test]
    fn porcelain_all_types() {
        let output = "A  added.rs\n M modified.rs\nUU conflict.rs\n";
        assert_eq!(parse_porcelain(output), (1, 1, 1));
    }

    // ── render_line1 ───────────────────────────────────────────────

    fn make_input(
        workspace: Option<Workspace>,
        cost: Option<Cost>,
        context_window: Option<ContextWindow>,
    ) -> StatusInput {
        StatusInput {
            model: Model {
                display_name: "Opus".to_string(),
            },
            workspace,
            cost,
            context_window,
            exceeds_200k_tokens: None,
        }
    }

    #[test]
    fn line1_model_and_dir() {
        let input = make_input(
            Some(Workspace {
                current_dir: "/home/user/myproject".to_string(),
                project_dir: None,
            }),
            None,
            None,
        );
        let line = render_line1(&input, None);
        assert!(line.contains("Opus"));
        assert!(line.contains("myproject"));
    }

    #[test]
    fn line1_no_workspace() {
        let input = make_input(None, None, None);
        let line = render_line1(&input, None);
        assert!(line.contains("?"));
    }

    #[test]
    fn line1_with_git() {
        let input = make_input(
            Some(Workspace {
                current_dir: "/home/user/proj".to_string(),
                project_dir: None,
            }),
            None,
            None,
        );
        let git = GitInfo {
            branch: "main".to_string(),
            staged: 2,
            modified: 1,
            conflicts: 0,
        };
        let line = render_line1(&input, Some(&git));
        assert!(line.contains("main"));
        assert!(line.contains("+2"));
        assert!(line.contains("~1"));
        assert!(!line.contains("!"));
    }

    #[test]
    fn line1_with_conflicts() {
        let input = make_input(
            Some(Workspace {
                current_dir: "/proj".to_string(),
                project_dir: None,
            }),
            None,
            None,
        );
        let git = GitInfo {
            branch: "feat".to_string(),
            staged: 0,
            modified: 0,
            conflicts: 3,
        };
        let line = render_line1(&input, Some(&git));
        assert!(line.contains("!3"));
    }

    #[test]
    fn line1_with_drift() {
        let input = make_input(
            Some(Workspace {
                current_dir: "/home/user/project/subdir".to_string(),
                project_dir: Some("/home/user/project".to_string()),
            }),
            None,
            None,
        );
        let line = render_line1(&input, None);
        assert!(line.contains("subdir"));
        assert!(line.contains("↩"));
        assert!(line.contains("project"));
    }

    // ── render_line2 ───────────────────────────────────────────────

    #[test]
    fn line2_defaults() {
        let input = make_input(None, None, None);
        let line = render_line2(&input);
        assert!(line.contains("0%"));
        assert!(line.contains("$0.00"));
        assert!(line.contains("0m 0s"));
    }

    #[test]
    fn line2_with_cost_and_duration() {
        let input = make_input(
            None,
            Some(Cost {
                total_cost_usd: Some(1.5),
                total_duration_ms: Some(125_000),
                total_lines_added: None,
                total_lines_removed: None,
            }),
            None,
        );
        let line = render_line2(&input);
        assert!(line.contains("$1.50"));
        assert!(line.contains("2m 5s"));
    }

    #[test]
    fn line2_with_lines_and_loc_per_dollar() {
        let input = make_input(
            None,
            Some(Cost {
                total_cost_usd: Some(2.0),
                total_duration_ms: Some(0),
                total_lines_added: Some(100),
                total_lines_removed: Some(50),
            }),
            None,
        );
        let line = render_line2(&input);
        assert!(line.contains("+100"));
        assert!(line.contains("-50"));
        assert!(line.contains("75 loc/$"));
    }

    #[test]
    fn line2_context_window_size() {
        let input = make_input(
            None,
            None,
            Some(ContextWindow {
                used_percentage: Some(45.0),
                context_window_size: Some(200_000),
            }),
        );
        let line = render_line2(&input);
        assert!(line.contains("45%"));
        assert!(line.contains("(200k)"));
    }

    #[test]
    fn line2_nudge_at_75() {
        let input = make_input(
            None,
            None,
            Some(ContextWindow {
                used_percentage: Some(75.0),
                context_window_size: None,
            }),
        );
        let line = render_line2(&input);
        assert!(line.contains("compact soon"));
    }

    #[test]
    fn line2_no_nudge_at_95() {
        let input = make_input(
            None,
            None,
            Some(ContextWindow {
                used_percentage: Some(95.0),
                context_window_size: None,
            }),
        );
        let line = render_line2(&input);
        assert!(!line.contains("compact soon"));
    }
}
