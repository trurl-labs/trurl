#!/usr/bin/env bash
#
# rename-to-trurlic.sh
#
# Renames every occurrence of "trurl" → "trurlic" / "Trurl" → "Trurlic"
# across the repo.  Run from the repo root (where Cargo.toml lives).
#
# Usage:
#   cd /path/to/trurl
#   bash rename-to-trurlic.sh        # live run
#   bash rename-to-trurlic.sh --dry   # preview only (no writes)
#
set -euo pipefail

DRY=false
[[ "${1:-}" == "--dry" ]] && DRY=true

# ── safety checks ────────────────────────────────────────────────────────────

if [[ ! -f Cargo.toml ]]; then
  echo "error: run this from the repo root (Cargo.toml not found)" >&2
  exit 1
fi

if ! grep -q 'name = "trurl"' Cargo.toml 2>/dev/null; then
  echo "error: Cargo.toml doesn't contain 'name = \"trurl\"' — already renamed?" >&2
  exit 1
fi

# ── collect files ────────────────────────────────────────────────────────────

FILES=$(find . -type f \( \
  -name '*.rs'   -o -name '*.toml' -o \
  -name '*.md'   -o -name '*.html' -o -name '*.css'  -o \
  -name '*.ts'   -o -name '*.json' -o \
  -name '*.yml'  -o -name '*.yaml' -o \
  -name 'Makefile' -o -name '.gitignore' \
\) ! -path './target/*' ! -path '*/node_modules/*')

# only files that actually contain "trurl" (case-insensitive)
HITS=$(echo "$FILES" | xargs grep -li 'trurl' 2>/dev/null || true)
COUNT=$(echo "$HITS" | grep -c . || true)

echo "Found $COUNT files containing 'trurl'."

# ── dry-run mode ─────────────────────────────────────────────────────────────

if $DRY; then
  echo ""
  echo "Dry run — files that would be modified:"
  echo "$HITS" | sed 's/^/  /'
  echo ""
  echo "Run without --dry to apply changes."
  exit 0
fi

# ── apply rename ─────────────────────────────────────────────────────────────

echo "Applying rename…"

# Pass 1: trurl → trurlic (lowercase — catches crate, binary, .trurl/, paths,
#          user-agent, thread names, field names like trurl_version, URLs)
echo "$HITS" | xargs sed -i'' -e 's/trurl/trurlic/g'

# Pass 2: Trurl → Trurlic (capitalized — display name, AI persona, docs)
echo "$HITS" | xargs sed -i'' -e 's/Trurl/Trurlic/g'

# Pass 3: TRURL → TRURLIC (all-caps — env vars like TRURL_DEBUG, constants)
echo "$HITS" | xargs sed -i'' -e 's/TRURL/TRURLIC/g'

# ── fix Lem reference ────────────────────────────────────────────────────────
# The literary character is "Trurl" (not "Trurlic"), so restore the original
# name in the README attribution line.

if [[ -f README.md ]]; then
  sed -i'' -e 's|Named after Trurlic from Stanisław Lem|Named after Trurl from Stanisław Lem|' README.md
  # Add a note explaining the spelling if not already present
  sed -i'' -e 's|the constructor who thinks deeply about what he builds before building it\.$|the constructor who thinks deeply about what he builds before building it. (Spelled *trurlic* to avoid conflict with curl'\''s [`trurl`](https://github.com/curl/trurl) URL tool.)|' README.md
fi

# ── Cargo.lock ───────────────────────────────────────────────────────────────
# Sed on Cargo.lock is fragile (checksums, hashes). Delete and let cargo
# regenerate cleanly on next build.

if [[ -f Cargo.lock ]]; then
  rm -f Cargo.lock
  echo "Cargo.lock removed — will regenerate on next cargo build/check."
fi

# ── summary ──────────────────────────────────────────────────────────────────

# verify nothing was missed
REMAINING=$(echo "$FILES" | xargs grep -l 'trurl' 2>/dev/null | xargs grep -L 'trurlic' 2>/dev/null || true)

echo ""
echo "Done. $COUNT files updated."
echo ""
echo "What changed:"
echo "  Cargo.toml    name/bin/lib = \"trurlic\""
echo "  CLI           trurlic init / trurlic design / trurlic serve …"
echo "  Store dir     .trurlic/"
echo "  Schema        trurlic_version"
echo "  Config        ~/.config/trurlic/config.toml"
echo "  MCP server    \"name\": \"trurlic\""
echo "  User-agent    trurlic/<version>"
echo "  Env vars      TRURLIC_DEBUG"
echo "  URLs          trurlic-labs/trurlic, trurlic.dev"
echo "  Frontend      <title>trurlic map</title>"
echo "  Docs          README, CONTRIBUTING, SECURITY, CLAUDE.md"
echo ""

if [[ -n "$REMAINING" ]]; then
  echo "⚠  Files with bare 'trurl' remaining (check manually):"
  echo "$REMAINING" | sed 's/^/  /'
else
  echo "✓  No bare 'trurl' remaining — clean rename."
fi

echo ""
echo "Next steps:"
echo "  1. git diff                   # review changes"
echo "  2. cargo check                # verify it compiles (regenerates Cargo.lock)"
echo "  3. cargo test                 # run tests"
echo "  4. git add -A && git commit -m 'rename: trurl → trurlic'"
echo "  5. Rename repo on GitHub:  Settings → General → Repository name → trurlic"
echo "  6. git remote set-url origin git@github.com:trurlic-labs/trurlic.git"
echo "  7. git push"
