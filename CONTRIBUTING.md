# Contributing

First: this is a single-file Rust binary that formats a status bar. You may want to reconsider your priorities.

Still here? Fine.

## Before You Write Code

Open an issue. Describe what you want to change and why. This exists so we can agree it's a good idea before you spend an afternoon on it and I spend an afternoon reviewing it. If the idea is bad, better to find out in a GitHub comment than in a diff.

## Pull Requests

Branch off main, make your change, open a PR that references the issue. Keep it small. The entire project is one file; a PR that touches more than that file is probably doing too much.

If you're adding behavior, add a JSON example in the PR description that demonstrates it. There's no test suite (see: single hobby binary), so the review doubles as QA.

## Code Style

Run `cargo fmt` before pushing. `cargo clippy` too, and don't submit with warnings unless you enjoy explaining them.

## What's in Scope

Rendering improvements, new status fields from the Claude Code JSON protocol, better git info. Things that make the two lines more useful without making them longer.

## What's Not in Scope

Config files, feature flags, plugin systems, or anything that requires reading from disk at runtime. It reads stdin, it writes stdout, it exits. That's the product.
