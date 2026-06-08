// ── Graph data (mirrors REST /api/graph response) ──────────────────────────

export interface GraphSnapshot {
  project: { name: string; description: string };
  components: ComponentNode[];
  decisions: DecisionNode[];
  patterns: PatternNode[];
  edges: EdgeData[];
  layout_version: number;
}

export interface ComponentNode {
  name: string;
  description: string;
  position: { x: number; y: number } | null;
  pinned: boolean;
  decision_count: number;
  pattern_count: number;
}

export interface DecisionNode {
  name: string;
  component: string;
  choice: string;
  reason: string;
  tags: string[];
  created: string;
  alternatives: string[];
}

export interface PatternNode {
  name: string;
  description: string;
  decisions: string[];
  components: string[];
}

export interface EdgeData {
  from: string;
  to: string;
  kind: string;
}

// ── Render-side types ──────────────────────────────────────────────────────

export interface RenderNode {
  name: string;
  kind: 'component' | 'decision' | 'pattern';
  x: number;
  y: number;
  w: number;
  h: number;
  pinned: boolean;
  /** Component-only fields */
  description?: string;
  decisionCount?: number;
  patternCount?: number;
  /** Decision-only fields */
  component?: string;
  choice?: string;
  reason?: string;
  tags?: string[];
}

export interface RenderEdge {
  from: string;
  to: string;
  kind: string;
}

// ── WebSocket events ───────────────────────────────────────────────────────

export interface WsEvent {
  type: string;
  [key: string]: unknown;
}

// ── Camera ─────────────────────────────────────────────────────────────────

export interface Viewport {
  x: number;
  y: number;
  w: number;
  h: number;
}
