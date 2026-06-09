//! Shared file watcher for live reload of `.trurlic/` changes.
//!
//! Both the MCP server and the map server need to detect external
//! changes to `.trurlic/` (CLI writes, manual edits, git checkout) and
//! reload state from disk. This module provides the shared
//! watch → filter → debounce → reload → drain loop. Consumers supply
//! a callback that receives the freshly loaded [`ProjectState`]; all
//! watcher plumbing is handled here.
//!
//! Events inside `.state/` (tmp files, lock, sessions) are ignored —
//! they are transient and never affect the graph.

use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use super::{STATE_DIR, Store};
use crate::store::ProjectState;

// ── Guard ────────────────────────────────────────────────────────────────────

/// Handle that keeps the watcher alive. Dropping stops the watch.
///
/// When dropped, the internal [`RecommendedWatcher`] is dropped, which
/// destroys the event callback and closes the channel sender. The
/// watcher thread sees `Disconnected` on its next `recv` and exits.
pub(crate) struct WatcherGuard {
    _watcher: RecommendedWatcher,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Spawn a background thread that watches `.trurlic/` and calls
/// `on_change` with a freshly loaded [`ProjectState`] whenever
/// relevant files change on disk.
///
/// `debounce` controls how long to batch events before reloading.
/// Lower values give faster UI updates; higher values coalesce
/// multi-file operations (e.g. `git checkout`).
///
/// The callback runs on the watcher thread with no locks held.
/// It is responsible for swapping the new state into whatever
/// shared structure the consumer uses.
///
/// Failure to create the watcher is non-fatal — the caller should
/// log the error and continue without live reload.
pub(crate) fn spawn(
    store_root: &Path,
    debounce: Duration,
    thread_name: &str,
    on_change: impl Fn(ProjectState) + Send + 'static,
) -> Result<WatcherGuard, String> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result {
                let _ = tx.send(event);
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("failed to create file watcher: {e}"))?;

    watcher
        .watch(store_root, RecursiveMode::Recursive)
        .map_err(|e| format!("failed to watch {}: {e}", store_root.display()))?;

    let store = Store::at(store_root.to_path_buf());
    let state_dir = store_root.join(STATE_DIR);

    thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || watch_loop(&store, &state_dir, debounce, rx, on_change))
        .map_err(|e| format!("failed to spawn watcher thread: {e}"))?;

    Ok(WatcherGuard { _watcher: watcher })
}

// ── Internals ──────────────────────────────────────────────────────────────

/// Event loop: block → filter → debounce → reload → callback → drain → repeat.
fn watch_loop(
    store: &Store,
    state_dir: &Path,
    debounce: Duration,
    rx: mpsc::Receiver<notify::Event>,
    on_change: impl Fn(ProjectState),
) {
    loop {
        // Block until an event arrives.
        let event = match rx.recv() {
            Ok(e) => e,
            Err(_) => return, // channel closed — server shutting down
        };

        // Skip events inside .state/ (tmp files, lock, sessions).
        if !is_relevant(&event, state_dir) {
            continue;
        }

        // Debounce: drain all events that arrive within the window.
        debounce_events(&rx, debounce);

        // Full reload: parse all files with no locks held, then hand
        // the new state to the consumer callback.
        match store.load_state() {
            Ok(new_state) => on_change(new_state),
            Err(e) => eprintln!("trurlic: watcher reload failed: {e}"),
        }

        // Drain events that arrived during reload — they reflect the state
        // we just loaded. Without this, a single CLI write triggers two
        // reloads: one from the debounced events, one from events that
        // arrived during the ~150ms load_state.
        drain_pending(&rx);
    }
}

/// Returns `true` if any event path is outside `.state/`.
fn is_relevant(event: &notify::Event, state_dir: &Path) -> bool {
    event.paths.iter().any(|p| !p.starts_with(state_dir))
}

/// Wait for the debounce window, consuming all events that arrive.
fn debounce_events(rx: &mpsc::Receiver<notify::Event>, duration: Duration) {
    let deadline = Instant::now() + duration;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        match rx.recv_timeout(remaining) {
            Ok(_) => continue,
            Err(mpsc::RecvTimeoutError::Timeout) => return,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Consume all events currently queued without blocking.
fn drain_pending(rx: &mpsc::Receiver<notify::Event>) {
    while rx.try_recv().is_ok() {}
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn event_at(paths: &[&str]) -> notify::Event {
        let mut e = notify::Event::new(notify::EventKind::Any);
        e.paths = paths.iter().map(PathBuf::from).collect();
        e
    }

    // ── is_relevant ─────────────────────────────────────────────────────

    #[test]
    fn relevant_for_component_file() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(is_relevant(
            &event_at(&["/repo/.trurlic/components/auth.toml"]),
            &sd,
        ));
    }

    #[test]
    fn relevant_for_graph_toml() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(is_relevant(&event_at(&["/repo/.trurlic/graph.toml"]), &sd));
    }

    #[test]
    fn relevant_for_project_toml() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(is_relevant(
            &event_at(&["/repo/.trurlic/project.toml"]),
            &sd
        ));
    }

    #[test]
    fn irrelevant_for_lock_file() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(!is_relevant(
            &event_at(&["/repo/.trurlic/.state/lock"]),
            &sd
        ));
    }

    #[test]
    fn irrelevant_for_tmp_file() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(!is_relevant(
            &event_at(&["/repo/.trurlic/.state/tmp/0_auth.toml"]),
            &sd,
        ));
    }

    #[test]
    fn irrelevant_for_session_file() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(!is_relevant(
            &event_at(&["/repo/.trurlic/.state/sessions/auth.json"]),
            &sd,
        ));
    }

    #[test]
    fn relevant_if_any_path_outside_state() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(is_relevant(
            &event_at(&[
                "/repo/.trurlic/.state/lock",
                "/repo/.trurlic/decisions/use-jwt.toml",
            ]),
            &sd,
        ));
    }

    #[test]
    fn irrelevant_for_empty_paths() {
        let sd = PathBuf::from("/repo/.trurlic/.state");
        assert!(!is_relevant(&event_at(&[]), &sd));
    }

    // ── drain_pending ───────────────────────────────────────────────────

    #[test]
    fn drain_pending_empties_channel() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..5 {
            tx.send(notify::Event::new(notify::EventKind::Any)).ok();
        }
        drain_pending(&rx);
        assert!(rx.try_recv().is_err());
    }
}
