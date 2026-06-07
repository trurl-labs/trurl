import type { RenderNode, RenderEdge } from './types';

/**
 * Simple force-directed layout. Runs a fixed number of iterations on
 * init, then only ticks when nodes are dragged (interactive mode).
 *
 * Forces:
 * - Repulsion: Coulomb between all node pairs
 * - Edge spring: Hooke's law along edges
 * - Center gravity: weak pull toward (0,0)
 */
export class ForceLayout {
  private repulsion = 8000;
  private springK = 0.005;
  private springLen = 250;
  private gravity = 0.01;
  private damping = 0.9;
  private vx: Map<string, number> = new Map();
  private vy: Map<string, number> = new Map();

  /** Run `iterations` ticks on the given nodes and edges. */
  run(nodes: Map<string, RenderNode>, edges: RenderEdge[], iterations: number): void {
    // Initialize velocities for new nodes.
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

  /** Single simulation step. */
  private tick(nodes: Map<string, RenderNode>, edges: RenderEdge[]): void {
    const fx = new Map<string, number>();
    const fy = new Map<string, number>();
    for (const name of nodes.keys()) {
      fx.set(name, 0);
      fy.set(name, 0);
    }

    // Repulsion (all pairs).
    const nodeArr = [...nodes.values()];
    for (let i = 0; i < nodeArr.length; i++) {
      for (let j = i + 1; j < nodeArr.length; j++) {
        const a = nodeArr[i];
        const b = nodeArr[j];
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

    // Edge springs.
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

    // Center gravity.
    for (const n of nodeArr) {
      fx.set(n.name, (fx.get(n.name) ?? 0) - n.x * this.gravity);
      fy.set(n.name, (fy.get(n.name) ?? 0) - n.y * this.gravity);
    }

    // Apply forces.
    for (const n of nodeArr) {
      if (n.pinned) continue;
      const vxOld = this.vx.get(n.name) ?? 0;
      const vyOld = this.vy.get(n.name) ?? 0;
      const vxNew = (vxOld + (fx.get(n.name) ?? 0)) * this.damping;
      const vyNew = (vyOld + (fy.get(n.name) ?? 0)) * this.damping;
      this.vx.set(n.name, vxNew);
      this.vy.set(n.name, vyNew);
      n.x += vxNew;
      n.y += vyNew;
    }
  }
}
