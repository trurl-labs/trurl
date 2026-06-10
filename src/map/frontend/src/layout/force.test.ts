import { describe, it, expect } from 'vitest';
import { ForceLayout } from './force';
import type { RenderNode, RenderEdge } from '../types';

function makeNode(name: string, x = 0, y = 0, w = 200, h = 60): RenderNode {
  return { name, kind: 'component', x, y, w, h, pinned: false };
}

describe('ForceLayout', () => {
  it('separates overlapping nodes', () => {
    const nodes = new Map<string, RenderNode>();
    nodes.set('a', makeNode('a', 0, 0));
    nodes.set('b', makeNode('b', 10, 10));
    const layout = new ForceLayout();
    layout.run(nodes, [], 50);
    const a = nodes.get('a')!;
    const b = nodes.get('b')!;
    // After layout, nodes should not overlap (accounting for padding).
    const dx = Math.abs(b.x - a.x);
    const dy = Math.abs(b.y - a.y);
    // At least one axis should exceed the sum of half-widths.
    expect(dx > a.w / 2 + b.w / 2 || dy > a.h / 2 + b.h / 2).toBe(true);
  });

  it('does not move pinned nodes', () => {
    const nodes = new Map<string, RenderNode>();
    const pinned = makeNode('pinned', 100, 100);
    pinned.pinned = true;
    nodes.set('pinned', pinned);
    nodes.set('free', makeNode('free', 110, 110));
    const layout = new ForceLayout();
    layout.run(nodes, [], 30);
    expect(nodes.get('pinned')!.x).toBe(100);
    expect(nodes.get('pinned')!.y).toBe(100);
  });

  it('connected nodes reach equilibrium near spring length', () => {
    const nodes = new Map<string, RenderNode>();
    nodes.set('a', makeNode('a', 0, 0));
    nodes.set('b', makeNode('b', 1000, 0));
    const edges: RenderEdge[] = [{ from: 'a', to: 'b', kind: 'connects_to' }];
    const layout = new ForceLayout();
    layout.run(nodes, edges, 300);
    const dist = Math.abs(nodes.get('b')!.x - nodes.get('a')!.x);
    // Should converge to roughly the spring length (400) ± 200.
    expect(dist).toBeGreaterThan(200);
    expect(dist).toBeLessThan(700);
  });

  it('collision avoidance prevents AABB overlap after layout', () => {
    const nodes = new Map<string, RenderNode>();
    // Place 5 nodes at the origin — maximum overlap.
    for (let i = 0; i < 5; i++) {
      nodes.set(`n${i}`, makeNode(`n${i}`, 0, 0));
    }
    const layout = new ForceLayout();
    layout.run(nodes, [], 100);

    // Check no pair overlaps (with a small tolerance).
    const arr = [...nodes.values()];
    for (let i = 0; i < arr.length; i++) {
      for (let j = i + 1; j < arr.length; j++) {
        const a = arr[i];
        const b = arr[j];
        const overlapX = a.w / 2 + b.w / 2 - Math.abs(b.x - a.x);
        const overlapY = a.h / 2 + b.h / 2 - Math.abs(b.y - a.y);
        const overlapping = overlapX > 5 && overlapY > 5;
        expect(overlapping).toBe(false);
      }
    }
  });
});
