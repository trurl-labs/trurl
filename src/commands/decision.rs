use std::path::Path;

use chrono::Utc;

use crate::store::schema::{Decision, DecisionFile, EdgeEntry, EdgeKind, NodeEntry, NodeKind};
use crate::store::{self};
use crate::{Error, Result};

use super::{open_store_mut, slugify, unique_decision_stem, validate_mutation};

pub fn decide(
    cwd: &Path,
    component: &str,
    choice: &str,
    reason: &str,
    supersedes: Option<&str>,
    alternatives: &[String],
) -> Result<()> {
    if component != "project" && !store::is_valid_kebab_case(component) {
        return Err(Error::InvalidName(component.into()));
    }

    let (store, lock, mut state) = open_store_mut(cwd)?;

    if component != "project" && !state.components.contains_key(component) {
        return Err(Error::Validation(format!(
            "component `{component}` does not exist"
        )));
    }

    if let Some(sup) = supersedes {
        if !state.decisions.contains_key(sup) {
            return Err(Error::Validation(format!(
                "decision `{sup}` does not exist (cannot supersede)"
            )));
        }
    }

    let stem = unique_decision_stem(&state.decisions, &slugify(choice))?;

    let decision = DecisionFile {
        decision: Decision {
            component: component.into(),
            choice: choice.into(),
            reason: reason.into(),
            alternatives: alternatives.to_vec(),
            created: Utc::now(),
        },
    };

    let write = store.prepare_write(&store.decision_path(&stem), &decision)?;
    let hash = write.content_hash();

    // Add node to graph index.
    state.graph_index.nodes.push(NodeEntry {
        name: stem.clone(),
        kind: NodeKind::Decision,
        tags: vec![],
        hash,
    });

    // Add BelongsTo edge.
    state.graph_index.edges.push(EdgeEntry {
        from: stem.clone(),
        to: component.into(),
        kind: EdgeKind::BelongsTo,
    });

    // Add Supersedes edge if applicable.
    if let Some(sup) = supersedes {
        state.graph_index.edges.push(EdgeEntry {
            from: stem.clone(),
            to: sup.into(),
            kind: EdgeKind::Supersedes,
        });
    }

    state.decisions.insert(stem.clone(), decision);
    validate_mutation(&state)?;

    store.commit_batch(&lock, vec![write], vec![], Some(&state.graph_index))?;
    println!("Recorded decision `{stem}`");
    Ok(())
}

pub fn remove_decision(cwd: &Path, name: &str) -> Result<()> {
    let (store, lock, mut state) = open_store_mut(cwd)?;

    if !state.decisions.contains_key(name) {
        return Err(Error::Validation(format!(
            "decision `{name}` does not exist"
        )));
    }

    // Warn about broken supersede chains via graph edges.
    let dependents: Vec<String> = state
        .graph_index
        .edges
        .iter()
        .filter(|e| e.to == name && e.kind == EdgeKind::Supersedes)
        .map(|e| e.from.clone())
        .collect();

    if !dependents.is_empty() {
        eprintln!(
            "warning: supersede chain broken — these decisions reference `{name}`: {}",
            dependents.join(", ")
        );
    }

    // Remove from state.
    state.decisions.remove(name);

    // Remove node and all edges involving this decision from graph index.
    state.graph_index.nodes.retain(|n| n.name != name);
    state
        .graph_index
        .edges
        .retain(|e| e.from != name && e.to != name);

    validate_mutation(&state)?;

    let removes = vec![store.decision_path(name)];
    store.commit_batch(&lock, vec![], removes, Some(&state.graph_index))?;
    println!("Removed decision `{name}`");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{add_component, init};
    use crate::store::Store;
    use crate::store::schema::EdgeKind;
    use tempfile::TempDir;

    #[test]
    fn decide_records_component_decision() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        decide(tmp.path(), "auth", "JWT with DPoP", "Stateless", None, &[]).unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let dec = store.read_decision("jwt-with-dpop").unwrap();
        assert_eq!(dec.decision.component, "auth");
        assert_eq!(dec.decision.choice, "JWT with DPoP");
    }

    #[test]
    fn decide_records_project_wide() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();

        decide(
            tmp.path(),
            "project",
            "Fail-closed on writes",
            "Never silently succeed",
            None,
            &[],
        )
        .unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let names = store.list_decisions().unwrap();
        assert_eq!(names.len(), 1);
        let dec = store.read_decision(&names[0]).unwrap();
        assert_eq!(dec.decision.component, "project");
    }

    #[test]
    fn decide_rejects_nonexistent_component() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();

        let err = decide(tmp.path(), "ghost", "x", "y", None, &[]).unwrap_err();
        match err {
            Error::Validation(msg) => assert!(msg.contains("ghost")),
            other => panic!("expected Validation, got: {other}"),
        }
    }

    #[test]
    fn decide_rejects_nonexistent_supersede_target() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        let err = decide(tmp.path(), "auth", "x", "y", Some("ghost"), &[]).unwrap_err();
        match err {
            Error::Validation(msg) => assert!(msg.contains("ghost")),
            other => panic!("expected Validation, got: {other}"),
        }
    }

    #[test]
    fn decide_supersedes_creates_edge() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        decide(tmp.path(), "auth", "Session cookies", "Simple", None, &[]).unwrap();
        decide(
            tmp.path(),
            "auth",
            "JWT tokens",
            "Stateless",
            Some("session-cookies"),
            &[],
        )
        .unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let state = store.load_state().unwrap();
        assert!(
            state
                .graph_index
                .edges
                .iter()
                .any(|e| e.from == "jwt-tokens"
                    && e.to == "session-cookies"
                    && e.kind == EdgeKind::Supersedes)
        );
    }

    #[test]
    fn decide_creates_belongs_to_edge() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        decide(tmp.path(), "auth", "Use JWT", "Stateless", None, &[]).unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let state = store.load_state().unwrap();
        assert!(
            state
                .graph_index
                .edges
                .iter()
                .any(|e| e.from == "use-jwt" && e.to == "auth" && e.kind == EdgeKind::BelongsTo)
        );
    }

    #[test]
    fn decide_records_alternatives() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        let alts = vec![
            "Session cookies — rejected: requires server-side state".into(),
            "Opaque tokens — rejected: introspection overhead".into(),
        ];
        decide(
            tmp.path(),
            "auth",
            "JWT with DPoP",
            "Stateless",
            None,
            &alts,
        )
        .unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let dec = store.read_decision("jwt-with-dpop").unwrap();
        assert_eq!(dec.decision.alternatives.len(), 2);
    }

    #[test]
    fn decide_deduplicates_filename() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        decide(tmp.path(), "auth", "Use Redis", "Fast", None, &[]).unwrap();
        decide(
            tmp.path(),
            "auth",
            "Use Redis",
            "Also for sessions",
            None,
            &[],
        )
        .unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let names = store.list_decisions().unwrap();
        assert_eq!(names, vec!["use-redis", "use-redis-2"]);
    }

    #[test]
    fn decide_sets_timestamp() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();

        let before = Utc::now();
        decide(tmp.path(), "auth", "JWT", "Stateless", None, &[]).unwrap();
        let after = Utc::now();

        let store = Store::discover(tmp.path()).unwrap();
        let dec = store.read_decision("jwt").unwrap();
        assert!(dec.decision.created >= before);
        assert!(dec.decision.created <= after);
    }

    #[test]
    fn decide_rejects_invalid_component_name() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();

        let err = decide(tmp.path(), "../escape", "x", "y", None, &[]).unwrap_err();
        assert!(matches!(err, Error::InvalidName(_)));
    }

    #[test]
    fn decide_allows_project_component() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();

        decide(tmp.path(), "project", "Test decision", "Testing", None, &[]).unwrap();
    }

    // ── remove decision ──────────────────────────────────────────────────

    #[test]
    fn remove_decision_deletes_file() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();
        decide(tmp.path(), "auth", "Use JWT", "Stateless", None, &[]).unwrap();

        remove_decision(tmp.path(), "use-jwt").unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        assert!(store.list_decisions().unwrap().is_empty());
    }

    #[test]
    fn remove_decision_cleans_up_edges() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();
        decide(tmp.path(), "auth", "Use JWT", "Stateless", None, &[]).unwrap();

        remove_decision(tmp.path(), "use-jwt").unwrap();

        let store = Store::discover(tmp.path()).unwrap();
        let state = store.load_state().unwrap();
        assert!(
            !state
                .graph_index
                .edges
                .iter()
                .any(|e| e.from == "use-jwt" || e.to == "use-jwt")
        );
    }

    #[test]
    fn remove_decision_rejects_nonexistent() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();

        let err = remove_decision(tmp.path(), "ghost").unwrap_err();
        match err {
            Error::Validation(msg) => assert!(msg.contains("ghost")),
            other => panic!("expected Validation, got: {other}"),
        }
    }

    #[test]
    fn remove_decision_warns_on_broken_supersede_chain() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        add_component(tmp.path(), "auth", None).unwrap();
        decide(tmp.path(), "auth", "Session cookies", "Simple", None, &[]).unwrap();
        decide(
            tmp.path(),
            "auth",
            "JWT tokens",
            "Stateless",
            Some("session-cookies"),
            &[],
        )
        .unwrap();

        // Removing session-cookies should succeed (with warning)
        remove_decision(tmp.path(), "session-cookies").unwrap();

        // jwt-tokens should still exist but its Supersedes edge is cleaned up
        let store = Store::discover(tmp.path()).unwrap();
        let state = store.load_state().unwrap();
        assert!(state.decisions.contains_key("jwt-tokens"));
        assert!(
            !state
                .graph_index
                .edges
                .iter()
                .any(|e| e.to == "session-cookies")
        );
    }
}
