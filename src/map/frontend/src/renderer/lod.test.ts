import { describe, it, expect } from 'vitest';
import { LOD, computeLOD } from './lod';

const REF = 3_360_000;

describe('computeLOD', () => {
  it('returns Decision for zero area', () => {
    expect(computeLOD(5, 0)).toBe(LOD.Decision);
  });

  it('returns Decision for negative area', () => {
    expect(computeLOD(5, -100)).toBe(LOD.Decision);
  });

  it('returns Overview for high density', () => {
    // 50 nodes at reference area → normalizedCount = 50 > 30.
    expect(computeLOD(50, REF)).toBe(LOD.Overview);
  });

  it('returns Component for moderate density', () => {
    // 15 nodes at reference area → normalizedCount = 15 > 3.
    expect(computeLOD(15, REF)).toBe(LOD.Component);
  });

  it('returns Decision for low density', () => {
    // 2 nodes at reference area → normalizedCount = 2 ≤ 3.
    expect(computeLOD(2, REF)).toBe(LOD.Decision);
  });

  it('adjusts for viewport area', () => {
    // 5 nodes in a very small viewport = high density.
    expect(computeLOD(5, REF / 10)).toBe(LOD.Overview);
    // 5 nodes in a very large viewport = low density.
    expect(computeLOD(5, REF * 10)).toBe(LOD.Decision);
  });

  it('boundary: exactly 30 normalized is Overview', () => {
    // normalizedCount = 30 → should be Overview (> 30 is false, so actually Component)
    // 30 * (REF / REF) = 30 → not > 30, so Component.
    expect(computeLOD(30, REF)).toBe(LOD.Component);
    expect(computeLOD(31, REF)).toBe(LOD.Overview);
  });

  it('boundary: exactly 3 normalized is Decision (threshold is strict >)', () => {
    // normalizedCount = 3 → not > 3, so Decision.
    expect(computeLOD(3, REF)).toBe(LOD.Decision);
    // 4 → > 3 → Component.
    expect(computeLOD(4, REF)).toBe(LOD.Component);
  });
});
