# Contributing

First: this is a single-file Rust binary that formats a status bar. You may want to reconsider your priorities.

Still here? Fine.

## Before You Write Code

Open an issue. Describe what you want to change and why. This exists so we can agree it's a good idea before you spend an afternoon on it and I spend an afternoon reviewing it. If the idea is bad, better to find out in a GitHub comment than in a diff.

## Pull Requests

Branch off main, make your change, open a PR that references the issue. Keep it small. The entire project is one file; a PR that touches more than that file is probably doing too much.

If you're adding behavior, add a JSON example in the PR description that demonstrates it and add tests to cover the new behavior.

## Code Style

Run `cargo fmt` before pushing. `cargo clippy` too, and don't submit with warnings unless you enjoy explaining them.

## What's in Scope

Rendering improvements, new status fields from the Claude Code JSON protocol, better git info. Things that make the two lines more useful without making them longer.

## What's Not in Scope

Config files, feature flags, plugin systems, or anything that requires reading from disk at runtime. It reads stdin, it writes stdout, it exits. That's the product.

## Releases

Releases use [cargo-release](https://github.com/crate-ci/cargo-release) to bump `Cargo.toml`, commit, tag, and push in one step:

```bash
cargo install cargo-release

# Dry run (preview what will happen)
cargo release patch

# Ship it
cargo release patch --execute    # 0.2.0 -> 0.2.1
cargo release minor --execute    # 0.2.0 -> 0.3.0
cargo release major --execute    # 0.2.0 -> 1.0.0
```

Configuration lives in `release.toml`. Publishing to crates.io is disabled since distribution is via cargo-dist.

GitHub Actions builds binaries for multiple targets, generates checksums and an installer script, and creates a release with all artifacts. This happens automatically when you push a version tag matching `v[0-9]+.[0-9]+.[0-9]+*`.
