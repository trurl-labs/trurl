import type { Viewport } from './types';

/**
 * Camera manages the world→screen coordinate transform.
 * World origin is (0,0) at graph center. Zoom > 1 means closer.
 */
export class Camera {
  /** Center of the viewport in world coordinates. */
  cx = 0;
  cy = 0;
  zoom = 1;
  /** Canvas pixel dimensions (set on resize). */
  screenW = 0;
  screenH = 0;

  private minZoom = 0.05;
  private maxZoom = 8;

  /** World → screen. */
  toScreenX(wx: number): number {
    return (wx - this.cx) * this.zoom + this.screenW / 2;
  }
  toScreenY(wy: number): number {
    return (wy - this.cy) * this.zoom + this.screenH / 2;
  }

  /** Screen → world. */
  toWorldX(sx: number): number {
    return (sx - this.screenW / 2) / this.zoom + this.cx;
  }
  toWorldY(sy: number): number {
    return (sy - this.screenH / 2) / this.zoom + this.cy;
  }

  /** Pan by screen-space delta. */
  pan(dsx: number, dsy: number): void {
    this.cx -= dsx / this.zoom;
    this.cy -= dsy / this.zoom;
  }

  /** Zoom centered on a screen-space point. */
  zoomAt(sx: number, sy: number, factor: number): void {
    const wx = this.toWorldX(sx);
    const wy = this.toWorldY(sy);
    this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, this.zoom * factor));
    // Adjust center so the world point stays under the cursor.
    this.cx = wx - (sx - this.screenW / 2) / this.zoom;
    this.cy = wy - (sy - this.screenH / 2) / this.zoom;
  }

  /** Animate to fit a bounding box with padding. */
  fitBounds(minX: number, minY: number, maxX: number, maxY: number, padding = 80): void {
    const bw = maxX - minX + padding * 2;
    const bh = maxY - minY + padding * 2;
    if (bw <= 0 || bh <= 0) return;
    this.cx = (minX + maxX) / 2;
    this.cy = (minY + maxY) / 2;
    this.zoom = Math.min(this.screenW / bw, this.screenH / bh, this.maxZoom);
    this.zoom = Math.max(this.zoom, this.minZoom);
  }

  /** Current visible world-space rectangle. */
  viewport(): Viewport {
    const hw = this.screenW / (2 * this.zoom);
    const hh = this.screenH / (2 * this.zoom);
    return { x: this.cx - hw, y: this.cy - hh, w: hw * 2, h: hh * 2 };
  }
}
