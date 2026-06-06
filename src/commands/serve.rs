use std::path::Path;

use crate::Result;

use super::discover_store;

pub fn serve(cwd: &Path) -> Result<()> {
    let store = discover_store(cwd)?;
    let state = store.load_state()?;

    let issues = state.validate();
    if !issues.is_empty() {
        eprintln!(
            "warning: .trurl/ has {} consistency issue(s) — run `trurl check`",
            issues.len()
        );
    }

    eprintln!(
        "trurl: serving {} ({} components, {} decisions)",
        state.project.project.name,
        state.components.len(),
        state.decisions.len(),
    );

    crate::mcp::run_server(store.root())
}
