# Starline

A single-binary Claude Code status line written in Rust. Reads JSON from stdin, prints ANSI-colored text to stdout.

## Build

```bash
cargo build --release          # standard build
dist build                     # release artifacts (uses [profile.dist])
```

## Test

No test suite yet. Validate manually with mock JSON piped to the binary:

```bash
echo '{"model":{"display_name":"Opus"},"workspace":{"current_dir":"/tmp","project_dir":"/tmp"},"cost":{"total_cost_usd":0.12,"total_duration_ms":134000,"total_lines_added":50,"total_lines_removed":10},"context_window":{"used_percentage":75,"context_window_size":200000}}' | ./target/release/starline
```

## Release

Releases are automated via `dist` (cargo-dist). Push a version tag to trigger cross-platform builds on GitHub Actions:

```bash
# bump version in Cargo.toml first
git tag v0.x.x
git push --tags
```

Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`

## Project structure

Single file: `src/main.rs`. No modules, no lib crate.

## Code patterns

- **All fields from Claude Code are `Option` with `#[serde(default)]`** — the JSON contract is loose, fields may be null or absent.
- **`run()` returns `Result`, `main()` catches errors** — never exit non-zero, that blanks the status bar. Print a fallback instead.
- **Pure rendering functions** — `render_line1`, `render_line2` take `&StatusInput` and return `String`. Easy to test, easy to extend.
- **Git via `gix` with `default-features = false`** — no OpenSSL, no HTTP transport. Pure Rust, cross-compiles cleanly.
- **ANSI colors as `const &str`** — no color library. Keep it simple.

## Adding new status fields

1. Add the field to the relevant struct (`StatusInput`, `Cost`, `ContextWindow`, etc.) with `Option<T>` + `#[serde(default)]`
2. Use it in `render_line1` or `render_line2` with `.unwrap_or()` fallbacks
3. Test with mock JSON that includes and omits the new field

See the [Claude Code statusline docs](https://code.claude.com/docs/en/statusline) for the full JSON schema.
