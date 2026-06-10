/** Semantic zoom levels per spec §Rendering Pipeline. */
export enum LOD {
  /** System overview: component boxes with decision count badge. */
  Overview = 0,
  /** Component detail: decision names listed inside component boxes. */
  Component = 1,
  /** Decision detail: full cards with choice, reason, tags. */
  Decision = 2,
}

/**
 * Reference viewport area in world-coordinate square-units.
 * Calibrated so that the density thresholds hold at a standard
 * 1920×1080 screen at zoom 1.0 where the world viewport is
 * approximately 2400×1400 units.
 */
const REFERENCE_AREA = 3_360_000;

/**
 * Determine LOD from information density, not raw count.
 *
 * Density = visibleCount / viewportWorldArea, normalized against a
 * reference viewport.  This means zooming into a sparse region of the
 * graph shows LOD 2 detail at a wider zoom than a dense cluster would,
 * matching spec §Rendering Pipeline ("LOD thresholds are derived from
 * information density").
 *
 * Thresholds are conservative: LOD 2 (decision cards) only activates
 * when ≤ 3 nodes fill the viewport, keeping the default project
 * overview clean and compact.
 *
 * @param visibleCount  Number of nodes whose bounds intersect the viewport.
 * @param viewportWorldArea  Viewport width × height in world coordinates.
 */
export function computeLOD(visibleCount: number, viewportWorldArea: number): LOD {
  if (viewportWorldArea <= 0) return LOD.Decision;

  const normalizedCount = visibleCount * (REFERENCE_AREA / viewportWorldArea);

  if (normalizedCount > 30) return LOD.Overview;
  if (normalizedCount > 3) return LOD.Component;
  return LOD.Decision;
}
