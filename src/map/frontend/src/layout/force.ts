import type { RenderNode, RenderEdge } from '../types';

/**
 * Force-directed layout with AABB collision separation.
 *
 * Four passes per tick:
 *   1. Repulsion  — inverse-square Coulomb between all node pairs.
 *   2. Springs    — Hooke's law on connected edges.
 *   3. Gravity    — gentle pull toward the origin.
 *   4. Collision  — AABB overlap resolution (the key to no-overlap).
 *
 * Collision uses the actual rendered node dimensions (w × h), not a
 * fixed radius, so LOD-expanded cards are accounted for.
 */
export class ForceLayout {
  /** Coulomb repulsion strength. Tuned for 5–50 node graphs. */
  private repulsion = 18_000;
  /** Hooke spring constant. */
  private springK = 0.004;
  /** Natural spring length (px). */
  private springLen = 400;
  /** Gravity strength (pull toward origin). */
  private gravity = 0.008;
  /** Velocity damping per tick. */
  private damping = 0.88;
  /** Padding between node AABBs during collision resolution (px). */
  private collisionPad = 24;

  private vx = new Map<string, number>();
  private vy = new Map<string, number>();

  run(nodes: Map<string, RenderNode>, edges: readonly RenderEdge[], iterations: number): void {
    // Ensure velocity maps cover all current nodes.
    for (const name of nodes.keys()) {
      if (!this.vx.has(name)) {
        this.vx.set(name, 0);
        this.vy.set(name, 0);
      }
    }
    for (let i = 0; i < iterations; i++) {
      this.tick(nodes, edges);
    }
  }

  private tick(nodes: Map<string, RenderNode>, edges: readonly RenderEdge[]): void {
    const fx = new Map<string, number>();
    const fy = new Map<string, number>();
    for (const name of nodes.keys()) {
      fx.set(name, 0);
      fy.set(name, 0);
    }

    const arr = [...nodes.values()];

    // ── 1. Repulsion ────────────────────────────────────────────────
    for (let i = 0; i < arr.length; i++) {
      for (let j = i + 1; j < arr.length; j++) {
        const a = arr[i];
        const b = arr[j];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = this.repulsion / (dist * dist);
        dx = (dx / dist) * force;
        dy = (dy / dist) * force;
        fx.set(a.name, (fx.get(a.name) ?? 0) - dx);
        fy.set(a.name, (fy.get(a.name) ?? 0) - dy);
        fx.set(b.name, (fx.get(b.name) ?? 0) + dx);
        fy.set(b.name, (fy.get(b.name) ?? 0) + dy);
      }
    }

    // ── 2. Springs ──────────────────────────────────────────────────
    for (const e of edges) {
      const a = nodes.get(e.from);
      const b = nodes.get(e.to);
      if (!a || !b) continue;
      let dx = b.x - a.x;
      let dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const force = this.springK * (dist - this.springLen);
      dx = (dx / dist) * force;
      dy = (dy / dist) * force;
      fx.set(a.name, (fx.get(a.name) ?? 0) + dx);
      fy.set(a.name, (fy.get(a.name) ?? 0) + dy);
      fx.set(b.name, (fx.get(b.name) ?? 0) - dx);
      fy.set(b.name, (fy.get(b.name) ?? 0) - dy);
    }

    // ── 3. Gravity ──────────────────────────────────────────────────
    for (const n of arr) {
      fx.set(n.name, (fx.get(n.name) ?? 0) - n.x * this.gravity);
      fy.set(n.name, (fy.get(n.name) ?? 0) - n.y * this.gravity);
    }

    // ── Apply forces ────────────────────────────────────────────────
    for (const n of arr) {
      if (n.pinned) continue;
      let nvx = (this.vx.get(n.name) ?? 0) + (fx.get(n.name) ?? 0);
      let nvy = (this.vy.get(n.name) ?? 0) + (fy.get(n.name) ?? 0);
      nvx *= this.damping;
      nvy *= this.damping;
      this.vx.set(n.name, nvx);
      this.vy.set(n.name, nvy);
      n.x += nvx;
      n.y += nvy;
    }

    // ── 4. Collision separation ──────────────────────────────────────
    this.separateOverlaps(arr);
  }

  /**
   * AABB overlap resolution. For each overlapping pair, push apart
   * along the axis of least overlap (shorter push = more stable).
   * Two passes per tick to handle transitive chains.
   */
  private separateOverlaps(nodes: RenderNode[]): void {
    const pad = this.collisionPad;

    for (let pass = 0; pass < 2; pass++) {
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const a = nodes[i];
          const b = nodes[j];

          // Half-extents including padding.
          const ahw = a.w / 2 + pad;
          const ahh = a.h / 2 + pad;
          const bhw = b.w / 2 + pad;
          const bhh = b.h / 2 + pad;

          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const overlapX = ahw + bhw - Math.abs(dx);
          const overlapY = ahh + bhh - Math.abs(dy);

          if (overlapX <= 0 || overlapY <= 0) continue;

          // Push along the axis with less overlap (more stable).
          if (overlapX < overlapY) {
            const shift = overlapX / 2;
            const sign = dx >= 0 ? 1 : -1;
            if (!a.pinned) a.x -= sign * shift;
            if (!b.pinned) b.x += sign * shift;
            // If one is pinned the other takes the full shift.
            if (a.pinned) b.x += sign * shift;
            if (b.pinned) a.x -= sign * shift;
          } else {
            const shift = overlapY / 2;
            const sign = dy >= 0 ? 1 : -1;
            if (!a.pinned) a.y -= sign * shift;
            if (!b.pinned) b.y += sign * shift;
            if (a.pinned) b.y += sign * shift;
            if (b.pinned) a.y -= sign * shift;
          }
        }
      }
    }
  }
}
