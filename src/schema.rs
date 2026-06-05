//! `.trurl/` TOML schema types.
//!
//! Every struct maps 1:1 to a file in `.trurl/`. All types derive
//! `Serialize` + `Deserialize` for TOML round-tripping and are
//! validated on every read and write.
//!
//! # Files
//!
//! | File                        | Type              |
//! |-----------------------------|-------------------|
//! | `project.toml`              | [`ProjectFile`]   |
//! | `components/<name>.toml`    | [`ComponentFile`] |
//! | `decisions/<name>.toml`     | [`DecisionFile`]  |

use serde::{Deserialize, Serialize};

// ── project.toml ─────────────────────────────────────────────────────────────

/// Root of `project.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectFile {
    /// Format version — checked on every CLI invocation.
    pub trurl_version: String,

    /// Project metadata.
    pub project: Project,
}

/// `[project]` table in `project.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    /// Human-readable project name.
    pub name: String,

    /// One-line description.
    pub description: String,
}

// ── components/<name>.toml ───────────────────────────────────────────────────

/// Root of a component file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentFile {
    /// Component definition.
    pub component: Component,
}

/// `[component]` table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Component {
    /// Kebab-case name, must match filename.
    pub name: String,

    /// What this component does.
    pub description: String,

    /// Names of components this one connects to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connects_to: Vec<String>,
}

// ── decisions/<name>.toml ────────────────────────────────────────────────────

/// Root of a decision file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionFile {
    /// Decision definition.
    pub decision: Decision,
}

/// `[decision]` table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Decision {
    /// Component this decision belongs to, or `"project"` for project-wide.
    pub component: String,

    /// What was decided.
    pub choice: String,

    /// Why — the programmer's reasoning.
    pub reason: String,

    /// Alternatives that were considered and rejected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,

    /// ISO 8601 timestamp of when this decision was recorded.
    pub created: String,

    /// Filename (without `.toml`) of the decision this supersedes, or empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub supersedes: String,
}

// ── Constants ────────────────────────────────────────────────────────────────

/// Current `.trurl/` format version. Written by `trurl init`, checked on
/// every CLI invocation.
pub const FORMAT_VERSION: &str = "0.1.0";

/// Directory name for the trurl store.
pub const STORE_DIR: &str = ".trurl";

/// Subdirectory for component files.
pub const COMPONENTS_DIR: &str = "components";

/// Subdirectory for decision files.
pub const DECISIONS_DIR: &str = "decisions";

/// Machine-local state directory (`.gitignore`'d).
pub const STATE_DIR: &str = ".state";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_round_trip() {
        let file = ProjectFile {
            trurl_version: "0.1.0".into(),
            project: Project {
                name: "my-project".into(),
                description: "Test project".into(),
            },
        };
        let serialized = toml::to_string_pretty(&file).expect("serialize");
        let deserialized: ProjectFile = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(file, deserialized);
    }

    #[test]
    fn component_round_trip() {
        let file = ComponentFile {
            component: Component {
                name: "auth".into(),
                description: "Authentication and token management".into(),
                connects_to: vec!["rate-limiter".into(), "database".into()],
            },
        };
        let serialized = toml::to_string_pretty(&file).expect("serialize");
        let deserialized: ComponentFile = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(file, deserialized);
    }

    #[test]
    fn decision_round_trip() {
        let file = DecisionFile {
            decision: Decision {
                component: "auth".into(),
                choice: "JWT with DPoP binding".into(),
                reason: "Stateless, no session store needed".into(),
                alternatives: vec!["Session cookies — rejected: requires server-side state".into()],
                created: "2025-06-01T10:30:00Z".into(),
                supersedes: String::new(),
            },
        };
        let serialized = toml::to_string_pretty(&file).expect("serialize");
        let deserialized: DecisionFile = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(file, deserialized);
    }

    #[test]
    fn component_empty_connects_to_omitted() {
        let file = ComponentFile {
            component: Component {
                name: "standalone".into(),
                description: "No connections".into(),
                connects_to: vec![],
            },
        };
        let serialized = toml::to_string_pretty(&file).expect("serialize");
        assert!(
            !serialized.contains("connects_to"),
            "empty connects_to should be omitted"
        );
    }

    #[test]
    fn decision_empty_supersedes_omitted() {
        let file = DecisionFile {
            decision: Decision {
                component: "auth".into(),
                choice: "Use Redis".into(),
                reason: "Fast".into(),
                alternatives: vec![],
                created: "2025-06-01T10:30:00Z".into(),
                supersedes: String::new(),
            },
        };
        let serialized = toml::to_string_pretty(&file).expect("serialize");
        assert!(
            !serialized.contains("supersedes"),
            "empty supersedes should be omitted"
        );
    }
}
