# starline

A Claude Code status line written in Rust, because the default one left too much money on the table without telling you about it.

Displays model, working directory, git branch, context window usage, session cost, and a passive judgment of your loc-per-dollar efficiency. Single binary, no runtime dependencies, zero OpenSSL.

## What It Looks Like

```
[claude-opus-4-6] 📁 myproject | 🌿 main +2 ~1
████████░░ (200k) 79% ⚡ compact soon | $1.24 | ⏱️ 14m 32s | +312 -89 (323 loc/$)
```

Line 1: model, directory, git status.
Line 2: context window, cost, time, lines changed, and how many lines of code you got per dollar (the number that haunts you).

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
  "statusCommand": "starline"
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
