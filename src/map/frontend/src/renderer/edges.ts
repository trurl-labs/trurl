/** Dash patterns per edge kind (empty array = solid). */
export const EDGE_DASH: Record<string, number[]> = {
  depends_on: [6, 4],
  constrains: [2, 3],
  supersedes: [8, 3, 2, 3],
};

/** Edge stroke color by kind, reading CSS variables with fallbacks. */
export function edgeColor(kind: string): string {
  if (kind === 'depends_on') return css('--edge-dep', '#5a7f5a');
  if (kind === 'constrains') return css('--edge-con', '#8f6c3a');
  return css('--edge', '#3a3f52');
}

function css(prop: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(prop).trim() || fallback;
}
