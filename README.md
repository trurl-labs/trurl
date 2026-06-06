# Trurl

Structured architectural decisions that constrain AI code generation.

![License](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square)
![Rust](https://img.shields.io/badge/rust-1.88%2B-orange?style=flat-square)

[Report a vulnerability](SECURITY.md) · [Contributing](CONTRIBUTING.md)

---

## The Problem

AI coding tools write clean code line by line, but make inconsistent architectural decisions across your project. Error handling in file A doesn't match file B. The cache strategy changes between modules. Every generation is locally correct but globally incoherent — and nobody notices because the programmer didn't make the decisions. The AI made them silently, differently each time.

The result: codebases that work but that nobody truly owns or understands.

## What Trurl Does

Trurl captures your architectural decisions and serves them as authoritative constraints to AI coding agents. You make the decisions. Your agent follows them.

**Three things, not one:**

1. **A file format** — `.trurl/` lives in your repo, git-tracked. Components, decisions, and patterns in TOML. Human-readable, hand-editable, portable.

2. **A CLI** — `trurl design rate-limiting` starts a Socratic design conversation in your terminal. The AI makes you think about tradeoffs, then records your decisions. Runs alongside your coding agent in a split terminal.

3. **An MCP server** — `trurl serve` lets any AI coding tool query your decisions. The agent asks "what decisions exist for this component?" and gets back a tailored spec with authoritative constraints. No static files to sync, always current.

Plus `trurl map` — an interactive visual of your system's architecture, decisions, and connections, in your browser.

## Install

```bash
cargo install trurl
```

## Quick Start

```bash
# Initialize in your project
trurl init

# Add components
trurl add component auth
trurl add component rate-limiter
trurl add component database
trurl add connection auth rate-limiter
trurl add connection rate-limiter database

# Design a component — Socratic conversation, decisions recorded
trurl design auth

# Start the MCP server — your coding agent queries this
trurl serve

# Open the interactive map
trurl map
```

## How It Works

```
You: "add rate limiting to the API"

Coding agent queries Trurl MCP:
  → "any decisions about rate limiting?"

Trurl responds with a tailored spec:
  RULES:
  - ALL error handling MUST use Result<T, AppError>
  - ALL persistent state MUST use Redis

  COMPONENT: rate-limiter
  - Per API key (consistent with auth boundary)
  - Redis-backed (consistent with session store)
  - 429 + retry-after header

  WHEN UNCERTAIN:
  STOP. Run trurl design <component> first.

Coding agent generates code following YOUR decisions.
```

When there are no decisions for something, the agent is told to stop and have you decide first. No silent pattern introduction. You stay in control.

## The `.trurl/` Directory

```
.trurl/
├── project.toml          # project metadata, format version
├── components/
│   ├── auth.toml         # component definition + connections
│   └── rate-limiter.toml
├── decisions/
│   ├── error-strategy.toml
│   └── rate-limit-storage.toml
└── .state/               # machine-local, gitignored
    └── sessions/         # conversation state for --continue
```

Everything in `.trurl/` (except `.state/`) is git-tracked, human-readable, and hand-editable. Your architectural decisions live alongside your code.

## Commands

| Command | Purpose |
|---------|---------|
| `trurl init` | Create `.trurl/` in your repo |
| `trurl add component <name>` | Add a component |
| `trurl add connection <from> <to>` | Connect components |
| `trurl rename component <old> <new>` | Rename a component, updating all references |
| `trurl remove component <name>` | Remove a component (refuses if decisions reference it) |
| `trurl remove decision <name>` | Remove a decision |
| `trurl design <component>` | Socratic design conversation |
| `trurl decide <component>` | Quick decision recording |
| `trurl serve` | Start MCP server for coding agents |
| `trurl map` | Open interactive map in browser |
| `trurl status` | Show components, decisions, consistency |
| `trurl check` | Validate `.trurl/` integrity |

## Philosophy

Named after Trurl from Stanisław Lem's "The Cyberiad" — the constructor who thinks deeply about what he builds, debates tradeoffs, and designs with craft and intention.

Every AI coding tool races to generate code faster. Trurl makes it more consistent, more understood, and more yours.

## License

Apache-2.0
