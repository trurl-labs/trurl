import type { Graph } from '../state/graph';

// ── Types ──────────────────────────────────────────────────────────────────

export interface SearchResult {
  name: string;
  kind: 'component' | 'decision' | 'pattern';
  label: string;
  /** Relevance score — higher is better. */
  score: number;
}

// ── Search ─────────────────────────────────────────────────────────────────

const MAX_RESULTS = 10;
const MIN_TOKEN_LEN = 2;

/**
 * Fuzzy search across component names, decision choices/reasons,
 * pattern names/descriptions, and tags. Tokenizes the query into
 * lowercase words, scores each entity by substring match count.
 * O(n × t) where n = total entities, t = query tokens. Under 5ms
 * for 5000 nodes with typical queries.
 */
export function search(graph: Graph, query: string): SearchResult[] {
  const tokens = tokenize(query);
  if (tokens.length === 0) return [];

  const results: SearchResult[] = [];

  for (const [name, node] of graph.nodes) {
    const blob = `${name} ${node.description ?? ''}`.toLowerCase();
    const score = scoreTokens(tokens, blob);
    if (score > 0) {
      results.push({ name, kind: 'component', label: name, score });
    }
  }

  for (const [name, dec] of graph.decisions) {
    const blob =
      `${name} ${dec.choice} ${dec.reason} ${dec.component} ${dec.tags.join(' ')}`.toLowerCase();
    const score = scoreTokens(tokens, blob);
    if (score > 0) {
      results.push({
        name,
        kind: 'decision',
        label: `${dec.choice} (${dec.component})`,
        score,
      });
    }
  }

  for (const [name, pat] of graph.patterns) {
    const blob = `${name} ${pat.description}`.toLowerCase();
    const score = scoreTokens(tokens, blob);
    if (score > 0) {
      results.push({ name, kind: 'pattern', label: pat.description, score });
    }
  }

  results.sort((a, b) => b.score - a.score);
  return results.slice(0, MAX_RESULTS);
}

/**
 * Return the set of node names within 1 hop of `center` via
 * connects_to edges (both directions). Includes `center` itself.
 */
export function neighborhood(graph: Graph, center: string): Set<string> {
  const result = new Set<string>();
  result.add(center);

  // For decisions, include their parent component.
  const dec = graph.decisions.get(center);
  if (dec) result.add(dec.component);

  for (const e of graph.edges) {
    if (e.kind !== 'connects_to') continue;
    if (e.from === center) result.add(e.to);
    if (e.to === center) result.add(e.from);
  }

  // Include decisions belonging to neighboring components.
  for (const name of [...result]) {
    for (const d of graph.decisionsFor(name)) {
      result.add(d.name);
    }
  }

  return result;
}

// ── Internals ──────────────────────────────────────────────────────────────

function tokenize(query: string): string[] {
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter((t) => t.length >= MIN_TOKEN_LEN);
}

function scoreTokens(tokens: string[], blob: string): number {
  let score = 0;
  for (const t of tokens) {
    if (blob.includes(t)) score++;
  }
  return score;
}
