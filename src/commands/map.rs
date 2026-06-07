use std::path::Path;

use crate::Result;
use crate::store::graph::Severity;

use super::discover_store;

pub fn map(cwd: &Path, port: Option<u16>, no_open: bool) -> Result<()> {
    let store = discover_store(cwd)?;
    let state = store.load_state()?;

    let errors = state
        .validate()
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    if errors > 0 {
        eprintln!("warning: .trurl/ has {errors} consistency issue(s) — run `trurl check`");
    }

    eprintln!(
        "trurl: map for {} ({} components, {} decisions, {} patterns)",
        state.project.project.name,
        state.components.len(),
        state.decisions.len(),
        state.patterns.len(),
    );

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| crate::Error::Io(std::io::Error::other(e)))?;

    rt.block_on(crate::map::start(store, state, port, no_open))
}
