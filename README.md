# Trurl

Structured architectural decisions that constrain AI code generation.

Named after [Trurl from Stanisław Lem's *The Cyberiad*](https://en.wikipedia.org/wiki/The_Cyberiad) — the constructor who thinks deeply about what he builds before building it.

## The Problem

AI coding tools produce locally correct code that is globally incoherent. Each generation picks its own patterns — error handling, state management, caching strategy — silently, differently each time. The programmer doesn't notice because they didn't make the decisions. The result: technical debt from invisible decisions nobody owns.

## The Solution

Trurl captures every architectural decision, makes the programmer engage with it, and serves decisions to AI coding agents as authoritative constraints via MCP.

**Three things, not one:**

1. **A file format** — `.trurl/` lives in your repo (like `.git/`). TOML files for components, decisions, and connections. Human-readable, git-tracked, hand-editable.

2. **A CLI** — `trurl design rate-limiting` starts a Socratic conversation in your terminal. You think, you decide, decisions are recorded. `trurl serve` starts an MCP server that any coding agent can query.

3. **A map** — `trurl map` opens an interactive visual of your system in the browser. Components, connections, decisions — explorable and editable.

## Status

**Phase 0 — Skeleton.** CLI structure defined, schema types in place, nothing implemented yet.

## Install

```bash
cargo install --path .
```

## Usage

```
trurl init                              # create .trurl/ in your project
trurl add component <name>              # define a component
trurl add connection <from> <to>        # connect components
trurl design <component>                # Socratic design conversation
trurl decide <component> --choice "..." --reason "..."  # quick decision
trurl serve                             # start MCP server
trurl map                               # open interactive map
trurl status                            # project overview
trurl check                             # validate consistency
```

## Development

```bash
make fmt       # format + auto-fix clippy
make check     # verify formatting + clippy (CI mode)
make test      # run all tests
make ci        # check + test with -Dwarnings
```

Requires Rust 1.88+. See `rust-toolchain.toml`.

## License

Apache-2.0
