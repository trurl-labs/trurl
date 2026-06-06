# Contributing to Trurl

Trurl is a developer tool that stores architectural decisions and serves them to AI coding agents. Contributions should maintain the same engineering standards the tool itself promotes: intentional decisions, clean code, and consistency.

## Reporting bugs and security issues

**Security vulnerabilities** — do not open a public issue. See [SECURITY.md](SECURITY.md).

**Bugs** — open a GitHub issue with your Trurl version (`trurl --version`), Rust version (`rustc --version`), platform, and minimal reproduction steps.

**Feature requests** — open an issue describing the use case before writing code. For changes to the `.trurl/` format, the MCP protocol, or the decision schema, wait for maintainer feedback — these have compatibility implications.

## Development setup

**Prerequisites:**

Rust 1.88+ (`rustup update stable`) and `make`.

**First-time setup:**

```bash
git clone https://github.com/trurl-labs/trurl.git
cd trurl
make setup    # installs git hooks
cargo build
cargo test
```

**Faster linker (optional):**

On Linux, `mold` cuts incremental link times significantly:

```bash
sudo apt install clang mold
```

Add to your personal `~/.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

## Running the tests

```bash
make test       # unit + integration tests
make check      # cargo fmt --check + clippy
make ci         # full local gate: check + test (run before pushing)
```

**Running a single test:**

```bash
cargo test -p trurl -- decision_store::tests::atomic_write
```

**Running with logs:**

```bash
RUST_LOG=debug cargo test 2>&1 | less
```

## Making changes

1. Fork the repo and create a branch from `main`: `git checkout -b feat/my-feature`.
2. Make your changes. Add tests for non-trivial features.
3. Run `make fmt` to auto-format.
4. Run `make ci` to verify everything passes.
5. Commit using [conventional commits](#commit-conventions).
6. Open a PR against `main`.

## Project structure

```
src/
├── main.rs              # CLI entry point (clap)
├── store/               # .trurl/ file operations, schema, validation
├── mcp/                 # MCP server, decision retrieval, spec assembly
├── conversation/        # Socratic design flow, Claude API client
├── map/                 # Map web server, API, static assets
└── common/              # Shared types, atomic writes, file locking
```

The internal dependency direction is intentional: `store` has no dependencies on other modules. `mcp` and `conversation` depend on `store`. `map` depends on `store`. Nothing depends on `main`.

## Working on the decision store

The `.trurl/` format is the foundation. Changes to schemas or file operations require extra care:

- All writes must use `write_atomic()` — write to `.state/tmp/`, validate, then rename. Never write directly to target files.
- All mutations validate full referential integrity before committing (decision references existing component, connection references existing endpoints, etc.).
- Schema changes must consider forward/backward compatibility. Bump `trurl_version` in `project.toml` for breaking format changes.
- Add property tests for any new validation logic.

## Working on the MCP server

The MCP server assembles specs from stored decisions. When changing response formats:

- The `brief` field must use authoritative language (MUST / DO NOT) — these are constraints, not suggestions.
- Test with a real coding agent (Claude Code or Cursor) to verify the response actually constrains generation.
- Selective retrieval must remain focused — return only relevant decisions, not everything.

## Working on the conversation engine

The Socratic design flow powers `trurl design`. When changing the question flow:

- Each user answer must write a decision to `.trurl/` immediately — no batching. If the process crashes, no answers are lost.
- Questions should make the user think, not give recommendations. "What matters more — latency or durability?" not "I suggest Redis."
- Test with `--continue` to ensure conversation state resumes correctly.

## Commit conventions

We use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

```
<type>(<scope>): <description>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`.

Scope is optional but helpful: `store`, `mcp`, `conversation`, `map`, `cli`.

```
feat(mcp): add related decisions from connected components to get_context response

fix(store): validate component references before writing decision

refactor(conversation): extract question generation into separate module

docs: add MCP integration guide for Claude Code
```

Breaking changes must include `BREAKING CHANGE:` in the footer.

## Pull request checklist

Before marking a PR ready for review:

- [ ] `make ci` passes locally
- [ ] New behavior has tests
- [ ] PR description explains *why*, not just *what*
- [ ] Schema changes bump `trurl_version` if breaking
- [ ] `CHANGELOG.md` updated under `[Unreleased]` for user-visible changes

## Code style

`make fmt` handles formatting and auto-fixable lints. Clippy is configured to deny warnings. Prefer explicit error types (`thiserror`) over `anyhow` in library code. Every public function has a doc comment. No `unwrap()` in production code paths — use proper error handling.

## Dogfooding

Trurl is built with Trurl. The repo has its own `.trurl/` directory with architectural decisions. When contributing a significant feature:

1. Check existing decisions with `trurl status`
2. If your change introduces a new pattern, run `trurl design` first
3. Record your architectural decisions — they help future contributors understand why
