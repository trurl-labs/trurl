/**
 * A single keyboard binding.
 *
 * `match` is tested against every keydown. If it returns `true`, `run`
 * is called and the event is consumed. Bindings are tested in order;
 * the first match wins.
 */
export interface KeyBinding {
  match: (e: KeyboardEvent) => boolean;
  run: (e: KeyboardEvent) => void;
}

/**
 * Keyboard dispatch table.
 *
 * Centralizes every keyboard shortcut in one place so they're
 * discoverable and testable. The host builds the binding list and
 * installs it via `attach()`.
 *
 * Spec §Keyboard Navigation: Ctrl+F, Ctrl+K, Escape, Ctrl+0, +/-,
 * arrows, Tab, Enter, Delete/Backspace, Ctrl+Z, Ctrl+Shift+Z.
 */
export class KeyboardDispatch {
  private bindings: KeyBinding[] = [];

  constructor(bindings: KeyBinding[]) {
    this.bindings = bindings;
  }

  /** Install the global keydown listener. Returns a cleanup function. */
  attach(): () => void {
    const handler = (e: KeyboardEvent) => this.handle(e);
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }

  private handle(e: KeyboardEvent): void {
    for (const b of this.bindings) {
      if (b.match(e)) {
        b.run(e);
        return;
      }
    }
  }
}

// ── Match helpers ───────────────────────────────────────────────────────

const META = (e: KeyboardEvent) => e.ctrlKey || e.metaKey;

export const Keys = {
  cmdK: (e: KeyboardEvent) => META(e) && e.key === 'k',
  search: (e: KeyboardEvent) => e.key === '/' || (META(e) && e.key === 'f'),
  undo: (e: KeyboardEvent) => META(e) && !e.shiftKey && e.key === 'z',
  redo: (e: KeyboardEvent) => META(e) && e.shiftKey && e.key === 'Z',
  escape: (e: KeyboardEvent) => e.key === 'Escape',
  zoomFit: (e: KeyboardEvent) => META(e) && e.key === '0',
  zoomIn: (e: KeyboardEvent) => e.key === '=' || e.key === '+',
  zoomOut: (e: KeyboardEvent) => e.key === '-',
  arrowLeft: (e: KeyboardEvent) => e.key === 'ArrowLeft',
  arrowRight: (e: KeyboardEvent) => e.key === 'ArrowRight',
  arrowUp: (e: KeyboardEvent) => e.key === 'ArrowUp',
  arrowDown: (e: KeyboardEvent) => e.key === 'ArrowDown',
  tab: (e: KeyboardEvent) => e.key === 'Tab',
  enter: (e: KeyboardEvent) => e.key === 'Enter',
  del: (e: KeyboardEvent) => e.key === 'Delete' || e.key === 'Backspace',
} as const;
