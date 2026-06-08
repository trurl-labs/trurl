import { describe, it, expect } from 'vitest';
import { LOD, computeLOD } from './lod';

// Reference area calibrated to 1920×1080 at zoom 1.0 (≈ 3,360,000 world sq-units).
const REF = 3_360_000;

describe('computeLOD', () => {
  it('returns Overview when density is high (many nodes, reference viewport)', () => {
    expect(computeLOD(41, REF)).toBe(LOD.Overview);
    expect(computeLOD(100, REF)).toBe(LOD.Overview);
    expect(computeLOD(5000, REF)).toBe(LOD.Overview);
  });

  it('returns Component at mid-range density', () => {
    expect(computeLOD(11, REF)).toBe(LOD.Component);
    expect(computeLOD(25, REF)).toBe(LOD.Component);
    expect(computeLOD(40, REF)).toBe(LOD.Component);
  });

  it('returns Decision when density is low (few nodes, reference viewport)', () => {
    expect(computeLOD(10, REF)).toBe(LOD.Decision);
    expect(computeLOD(5, REF)).toBe(LOD.Decision);
    expect(computeLOD(1, REF)).toBe(LOD.Decision);
  });

  it('returns Decision for zero visible nodes', () => {
    expect(computeLOD(0, REF)).toBe(LOD.Decision);
  });

  it('returns Decision for zero viewport area (degenerate)', () => {
    expect(computeLOD(100, 0)).toBe(LOD.Decision);
    expect(computeLOD(100, -1)).toBe(LOD.Decision);
  });

  // ── Density-normalization tests ────────────────────────────────────

  it('shows more detail in a larger viewport with the same node count', () => {
    // 20 nodes at reference area → Component (normalized 20).
    expect(computeLOD(20, REF)).toBe(LOD.Component);
    // Same 20 nodes in a viewport 3× the area → sparser (normalized ≈6.7) → Decision.
    expect(computeLOD(20, REF * 3)).toBe(LOD.Decision);
  });

  it('shows less detail in a smaller viewport with the same node count', () => {
    // 30 nodes at reference area → Component (normalized 30).
    expect(computeLOD(30, REF)).toBe(LOD.Component);
    // Same 30 nodes in a viewport half the area → denser (normalized 60) → Overview.
    expect(computeLOD(30, REF / 2)).toBe(LOD.Overview);
  });

  it('sparse region at any zoom level shows full detail', () => {
    // 3 nodes in a small viewport: normalized = 3 * (REF / (REF/10)) = 30 → Component.
    expect(computeLOD(3, REF / 10)).toBe(LOD.Component);
    // 3 nodes in a very large viewport: normalized = 3 * (REF / (REF*10)) = 0.3 → Decision.
    expect(computeLOD(3, REF * 10)).toBe(LOD.Decision);
  });
});
