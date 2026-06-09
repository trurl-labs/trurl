<p align="center">
  <img src="./banner.png" alt="Trurlic" height="100">
</p>

<p align="center">
  <b>Structured architectural decisions that constrain AI code generation.</b>
</p>

<p align="center">
  <a href="https://github.com/trurlic-labs/trurlic/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/trurlic-labs/trurlic/ci.yml?style=flat-square&label=CI" alt="CI"></a>
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/rust-1.88%2B-orange?style=flat-square" alt="Rust">
  <br>
  <a href="SECURITY.md">Report a vulnerability</a> · <a href="CONTRIBUTING.md">Contributing</a>
</p>

## The Problem

AI engineering is real leverage, but with a real cost.

You're shipping faster than ever, but you've stopped making the decisions that define your architecture - the AI makes them for you, silently, differently each time, and you approve because each diff looks reasonable in isolation. The speed is real. So is the cost: you're losing ownership of your own codebase, trading deep understanding for throughput, and accumulating technical debt that no linter will ever catch.

Six months in, you can't refactor without breaking things you didn't know existed. You can't onboard anyone because there's no design to explain — just a pile of locally-correct code with no global coherence. The system works. **You just don't own it**.

Trurlic fixes this. You make the architectural decisions. Trurlic records them, forces you to understand them, and feeds them as hard constraints to every AI-generated line of code. One decision graph, every agent follows it, nothing slips through.

## What Trurlic Does

Every architectural decision is captured in a queryable graph, understood by the programmer through forced engagement, and used to constrain AI code generation.

**A file format.** `.trurlic/` is like `.git/`. It lives in your repo, git-tracked. Contains a typed knowledge graph: components, decisions, patterns, and their relationships. TOML node files for human readability, a compiled edge index for fast traversal.

**An MCP server.** `trurlic serve` starts a local MCP server that any AI coding tool queries. The coding agent calls Trurlic to get context before implementing, to run design conversations when new patterns are needed, and to record decisions as they're made. You never leave your coding tool.

**A map.** `trurlic map` opens an interactive visualization of the architecture graph in the browser. Components, connections, decisions, patterns — explorable, editable, always in sync with `.trurlic/`.

**A CLI.** `trurlic design <component>` runs a Socratic design conversation — the AI asks you questions, you think through tradeoffs, and your answers become recorded decisions. `trurlic decide` records quick decisions from the terminal. Everything local, everything under 100ms.

Named after Trurl from Stanisław Lem's *The Cyberiad* — the constructor who thinks deeply about what he builds before building it. (Spelled *trurlic* to avoid conflict with curl's [`trurl`](https://github.com/curl/trurl) URL tool.)

## Install

```
cargo install trurlic
```

Requires Rust 1.88+.

## Quick Start

```bash
trurlic init
trurlic add component auth -d "Authentication and token management"
trurlic add component database -d "Persistence layer"
trurlic add connection auth database

# Record decisions directly
trurlic decide project --choice "Result<T, AppError> for all errors" --reason "Consistent error propagation"
trurlic decide auth --choice "JWT with DPoP binding" --reason "Stateless, no session store"

# Or run a guided design conversation
trurlic design auth

# Start the MCP server for AI coding agents
trurlic serve
```

## MCP Integration

`trurlic serve` exposes these tools over MCP (stdio transport):

| Tool | Purpose |
|------|---------|
| `advance` | Compute workflow state and return the next action — the orchestration hub |
| `get_context` | Tailored brief for a component: decisions, project rules, related constraints |
| `check_pattern` | Check if an approach is covered by existing decisions |
| `get_architecture` | Full system overview: components, connections, patterns |
| `get_design_prompt` | Structured prompt for design conversations (full/quick/learn/review) |
| `add_component` | Add a new component to the architecture graph |
| `add_connection` | Add a directional connection between components |
| `record_decision` | Record a decision with edges, tags, alternatives |
| `record_pattern` | Record a pattern synthesized from multiple decisions |
| `update_decision` | Amend (typo fix) or supersede (substantive change) |
| `remove_decision` | Remove with cascade awareness |
| `validate_consistency` | Full graph integrity check |

### Agent Workflow

Add to your `CLAUDE.md` (or equivalent):

```markdown
## Trurlic

This project uses Trurlic for architectural decisions.
The Trurlic MCP server enforces design-before-implementation.

### Workflow

Before implementing any task:

1. Call `check_pattern` with a description of what you're about to do.
   - If covered: call `get_context` and use the brief as constraints.
   - If not covered: continue to step 2.
2. Call `advance` with the component name.
3. Follow the returned `action` exactly.
4. After completing the action, call `advance` again.
5. Repeat until `ready: true`.
6. Call `get_context` for the implementation brief.
7. Implement, constrained by every decision in the brief.

When the user asks to learn or review:
call `advance` with `intent: "learn"` or `intent: "review"`.

### Comprehension Gates

When Trurlic's design prompt includes comprehension checkpoints,
ask the user to articulate their understanding in their own words.
The user explains — you validate. Do not explain on their behalf.
```

### What the Agent Sees

When the agent calls `get_context("rate-limiter")`:

```
RULES (inviolable — every generated line must respect these):
- ALL error handling MUST use Result<T, AppError>
- ALL persistent state MUST use Redis

COMPONENT: rate-limiter
- Per API key, consistent with auth boundary
- Redis sliding window, 60s buckets
- 429 + retry-after header, RFC 6585 compliant

PATTERNS:
- state-in-redis: shared pool via app state, no per-component connections

RELATED:
- auth: JWT with DPoP, rate limiter runs AFTER auth

OVERRIDE POLICY:
RULES are inviolable. Component decisions are strong defaults —
follow them unless the user explicitly revises them in a design session.
Never silently deviate from either.

WHEN UNCERTAIN:
STOP. This introduces a new pattern. Ask the user to design it first.
```

## Design Modes

`get_design_prompt` supports four modes:

| Mode | When | Depth |
|------|------|-------|
| `full` | New component or major feature | Multi-phase: scope → technical choices → pattern recognition → summary checkpoint. Dynamic concern tracking shows what's covered and what needs exploration. Comprehension gates after each decision. |
| `quick` | Small addition to existing component | Presents all active constraints for confirmation, then checks for new decisions. |
| `learn` | Studying existing design | All decisions with challenge questions. Probes for unrecorded decisions. No implementation. |
| `review` | Periodic health check | Decisions sorted oldest-first. "Does this still hold?" |

## The `.trurlic/` Directory

```
.trurlic/
├── project.toml                  # project metadata, format version
├── graph.toml                    # edge index + node hashes (git-tracked)
├── components/
│   ├── auth.toml
│   └── rate-limiter.toml
├── decisions/
│   ├── error-strategy.toml
│   └── rate-limit-storage.toml
├── patterns/
│   └── state-in-redis.toml
└── .state/                       # machine-local, gitignored
    ├── lock
    ├── tmp/
    └── sessions/
```

Node files are TOML. Edges live in `graph.toml`. Files are truth, index is derived — `trurlic check --rebuild` restores the index from node files.

### Graph Edges

| Edge | From → To | Meaning |
|------|-----------|---------|
| `belongs_to` | decision → component | Decision is about this component |
| `connects_to` | component → component | Data/control flow |
| `depends_on` | decision → decision | This decision assumes the target holds |
| `constrains` | decision → decision | Restricts options for the target |
| `supersedes` | decision → decision | Replaces the target (history preserved) |
| `member_of` | pattern → decision | Decision is part of this pattern |
| `applies_to` | pattern → component | Pattern applies to this component |

## CLI Reference

```
trurlic init                                  Create .trurlic/ in current directory
trurlic add component <name> [-d <desc>]      Define a component
trurlic add connection <from> <to>            Connect two components
trurlic rename component <old> <new>          Rename, updating all references
trurlic remove component <name>               Remove (refuses if decisions reference it)
trurlic remove decision <name>                Remove (refuses if depended on)
trurlic remove connection <from> <to>         Remove a connection
trurlic decide <component> --choice "..." --reason "..."
      [--supersede <name>] [-a "..."]       Quick decision recording
trurlic design <component>                    Socratic design conversation
      [--continue] [--revisit]
      [-p anthropic|openai|openrouter]
      [-m <model>]
trurlic serve                                 Start MCP server (stdio)
trurlic map [--port <n>] [--no-open]          Open interactive map in browser
trurlic status                                Component/decision/pattern counts
trurlic check                                 Validate graph integrity
trurlic check --rebuild                       Force-rebuild graph.toml from files
```

## API Key Configuration

Environment variables (checked first):
```
ANTHROPIC_API_KEY
OPENAI_API_KEY
OPENROUTER_API_KEY
```

Fallback: `~/.config/trurlic/config.toml` (must be `chmod 600`):
```toml
default_provider = "anthropic"
anthropic_api_key = "sk-ant-..."
```

Keys are zeroed from memory on drop and never appear in logs or error output.

## Design Principles

**Fail-closed on writes.** Every write validates the full graph in memory before touching disk. Dangling reference, cycle, schema violation → refuse with a clear error.

**Atomic writes.** Temp file, verify round-trip, rename. `graph.toml` renamed last as the commit point. Interrupted writes cleaned up on next startup.

**Offline-first.** Only `trurlic design` calls an LLM API. MCP, CLI, check, status — all local. The MCP server provides prompts; the coding agent's own LLM runs the conversation.

**Files are truth.** `graph.toml` is derived and rebuildable. Hand-edit a TOML file, run `trurlic check`, the graph reconciles.

**Live reload.** The MCP server watches `.trurlic/` for external changes and reloads automatically.

## Development

```bash
make setup        # git hooks
make ci           # fmt + clippy + test + audit (run before pushing)
make audit        # cargo deny check (advisories, licenses, bans)
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development guide.

## License

Apache-2.0
