# starline

A Claude Code status line written in Rust, because the default one left too much money on the table without telling you about it.

Displays model, working directory, git branch, context window usage, session cost, and a passive judgment of your loc-per-dollar efficiency. Single binary, minimal dependencies. Requires `git` on `PATH`.

## What It Looks Like

```
[claude-opus-4-6] 📁 myproject | 🌿 main +2 ~1
████████░░ (200k) 79% ⚡ compact soon | $1.24 | ⏱️ 14m 32s | +312 -89 (323 loc/$)
```

Line 1: model, directory, git status.
Line 2: context window, cost, time, lines changed, and how many lines of code you got per dollar (the number that haunts you).

## Status Elements

### Line 1: Identity and Location

`[model]` is the display name from Claude Code, rendered as received. No mapping, no aliasing. Whatever Anthropic decides to call the next model, that's what you'll see.

`📁 dir` shows the final path component of your current working directory. If you're in `/home/you/projects/api`, you see `api`. When `current_dir` and `project_dir` diverge, a drift warning appears: `⚠️ ↩ project_name`, showing the project root you've wandered from. This is easy to miss in a long session and surprisingly useful when you do.

`🌿 branch` is read from the local git repo via `git rev-parse` and `git status`. Named branches display as you'd expect. A detached HEAD shows the short commit hash. If the directory isn't a git repo (or `git` isn't installed), the entire git section is quietly omitted.

The counters after the branch name reflect working tree state:

- `+N` (green): staged files, ready to commit
- `~N` (yellow): modified but unstaged files
- `!N` (red): files with merge conflicts

Each counter only appears when its value is greater than zero. No news is good news; merge conflicts are the exception that proves the rule.

### Line 2: Resources and Efficiency

The context bar is a 10-character gauge of your context window consumption. Filled blocks represent `used_percentage / 10`, truncated to an integer. The bar color shifts with usage: green below 70%, yellow from 70% to 89%, red at 90% and above. When the window size is known, it appears in parentheses, bucketed to `200k`, `1M`, or the raw value in thousands.

Between 70% and 84%, a `⚡ compact soon` nudge appears. It disappears at 85%, not because the problem went away, but because nagging you at that point would be redundant.

`$cost` is `total_cost_usd` formatted to two decimal places. Always visible, even at zero, because the meter is running whether you look at it or not.

`⏱️ Xm Ys` is session duration derived from `total_duration_ms`. Minutes and seconds. If your session runs long enough to need an hours field, that's a you problem, not a starline problem.

`+added -removed` shows total lines changed across the session. Green for additions, red for removals. Hidden until at least one line has been touched, so a fresh session doesn't open with a wall of zeroes judging you.

`(N loc/$)` is the efficiency metric: total lines changed divided by total cost, rounded to the nearest integer. It requires both meaningful line changes and a cost above $0.001 to avoid dividing by dust. The number is descriptive, not prescriptive; some of your most expensive sessions will be the most valuable, and vice versa.

## Installation

Download the latest binary for your platform from [Releases](https://github.com/anoldguy/starline/releases/latest):

```bash
# macOS (Apple Silicon)
curl -L https://github.com/anoldguy/starline/releases/latest/download/starline-aarch64-apple-darwin.tar.gz | tar xz
sudo mv starline /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/anoldguy/starline/releases/latest/download/starline-x86_64-apple-darwin.tar.gz | tar xz
sudo mv starline /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/anoldguy/starline/releases/latest/download/starline-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv starline /usr/local/bin/
```

Or build from source if you don't trust binaries from strangers (reasonable):

```bash
cargo install --git https://github.com/anoldguy/starline
```

## Claude Code Configuration

Add to your `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "starline"
  }
}
```

Claude Code pipes a JSON blob to stdin on each status update. Starline reads it, renders two lines of ANSI output, and exits. That's the entire protocol.

Verify it works manually:

```bash
echo '{"model":{"display_name":"claude-opus-4-6"},"workspace":{"current_dir":"/tmp","project_dir":"/tmp"},"cost":{"total_cost_usd":0.12,"total_duration_ms":134000,"total_lines_added":50,"total_lines_removed":10},"context_window":{"used_percentage":75,"context_window_size":200000}}' | starline
```

If it prints two colored lines, you're done. If it prints `[starline]`, the JSON was malformed — which shouldn't happen since Claude Code generates it, but starline fails silently anyway because a broken status bar is worse than a wrong one.

## Platform Support

`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
