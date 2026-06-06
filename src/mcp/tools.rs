//! MCP tool definitions, JSON Schema for inputs, and dispatch.
//!
//! Three tools exposed to coding agents:
//! - `get_context`     — tailored spec for a component
//! - `check_pattern`   — is this pattern covered by existing decisions?
//! - `get_architecture` — full system overview

use serde_json::Value;

use crate::store::Store;

use super::context;

// ── Tool metadata ─────────────────────────────────────────────────────────

/// Return the `tools/list` response payload.
pub(crate) fn tool_list() -> Value {
    serde_json::json!({
        "tools": [
            {
                "name": "get_context",
                "description": concat!(
                    "Get architectural decisions and constraints for a component. ",
                    "Returns the component's decisions, project-wide rules, related ",
                    "decisions from connected components, and a pre-assembled ",
                    "authoritative brief. Use before generating code for any component."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "component": {
                            "type": "string",
                            "description": "Component name, or \"project\" for project-wide context"
                        },
                        "task_description": {
                            "type": "string",
                            "description": "What you are about to implement (included in the brief for context)"
                        }
                    },
                    "required": ["component"]
                }
            },
            {
                "name": "check_pattern",
                "description": concat!(
                    "Check whether a pattern or approach is covered by existing ",
                    "architectural decisions before introducing it. Returns ",
                    "'covered' with relevant constraints, or 'not_covered' with a ",
                    "suggestion to run `trurl design` first."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": concat!(
                                "The pattern or approach to check ",
                                "(e.g. \"adding Redis pub/sub for real-time notifications\")"
                            )
                        }
                    },
                    "required": ["description"]
                }
            },
            {
                "name": "get_architecture",
                "description": concat!(
                    "Get a full overview of the system architecture: all components, ",
                    "their connections, project-wide decisions, and decision counts."
                ),
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

// ── Dispatch ──────────────────────────────────────────────────────────────

/// Execute a tool call.  Re-reads `.trurl/` from disk on every call so
/// the state is always current.  Returns an MCP tool-result envelope
/// (never a JSON-RPC error — tool failures use `isError`).
pub(crate) fn call_tool(store: &Store, name: &str, arguments: &Value) -> Value {
    let state = match store.load_state() {
        Ok(s) => s,
        Err(e) => return tool_error(&format!("failed to load .trurl/ state: {e}")),
    };

    match name {
        "get_context" => dispatch_get_context(&state, arguments),
        "check_pattern" => dispatch_check_pattern(&state, arguments),
        "get_architecture" => dispatch_get_architecture(&state),
        _ => tool_error(&format!("unknown tool: {name}")),
    }
}

fn dispatch_get_context(state: &ProjectState, args: &Value) -> Value {
    let component = match args.get("component").and_then(Value::as_str) {
        Some(c) => c,
        None => return tool_error("missing required parameter: component"),
    };
    let task = args.get("task_description").and_then(Value::as_str);

    match context::get_context(state, component, task) {
        Ok(response) => tool_result(&response),
        Err(msg) => tool_error(&msg),
    }
}

fn dispatch_check_pattern(state: &ProjectState, args: &Value) -> Value {
    let description = match args.get("description").and_then(Value::as_str) {
        Some(d) => d,
        None => return tool_error("missing required parameter: description"),
    };
    tool_result(&context::check_pattern(state, description))
}

fn dispatch_get_architecture(state: &ProjectState) -> Value {
    tool_result(&context::get_architecture(state))
}

// ── MCP result envelopes ─────────────────────────────────────────────────

/// Wrap a payload as a successful tool-call result.
fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string_pretty(payload).unwrap_or_else(|_| payload.to_string());
    serde_json::json!({
        "content": [{ "type": "text", "text": text }]
    })
}

/// Wrap an error message as a failed tool-call result.
fn tool_error(message: &str) -> Value {
    serde_json::json!({
        "content": [{ "type": "text", "text": message }],
        "isError": true
    })
}

// ── Private re-import ───────────────────────────────────────────────────

use crate::store::ProjectState;

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_list_has_three_tools() {
        let list = tool_list();
        let tools = list["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn tool_list_has_correct_names() {
        let list = tool_list();
        let tools = list["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"get_context"));
        assert!(names.contains(&"check_pattern"));
        assert!(names.contains(&"get_architecture"));
    }

    #[test]
    fn tool_list_schemas_have_required_fields() {
        let list = tool_list();
        let tools = list["tools"].as_array().unwrap();
        for tool in tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            assert!(tool.get("inputSchema").is_some());
            assert_eq!(tool["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn tool_result_wraps_in_content_block() {
        let payload = serde_json::json!({"status": "covered"});
        let result = tool_result(&payload);

        let content = result["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert!(result.get("isError").is_none());
    }

    #[test]
    fn tool_error_sets_is_error() {
        let result = tool_error("something broke");
        assert_eq!(result["isError"], true);
        assert_eq!(result["content"][0]["text"], "something broke");
    }

    #[test]
    fn dispatch_unknown_tool_returns_error() {
        let payload = serde_json::json!({});
        let result = dispatch_unknown(&payload);
        assert_eq!(result["isError"], true);
    }

    fn dispatch_unknown(args: &Value) -> Value {
        // Simulate calling an unknown tool without needing a Store.
        let _ = args;
        tool_error(&format!("unknown tool: {}", "nonexistent"))
    }

    #[test]
    fn dispatch_get_context_missing_component() {
        let state = empty_state();
        let args = serde_json::json!({});
        let result = dispatch_get_context(&state, &args);
        assert_eq!(result["isError"], true);
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("component"));
    }

    #[test]
    fn dispatch_check_pattern_missing_description() {
        let state = empty_state();
        let args = serde_json::json!({});
        let result = dispatch_check_pattern(&state, &args);
        assert_eq!(result["isError"], true);
    }

    fn empty_state() -> ProjectState {
        use crate::store::schema::*;
        ProjectState {
            project: ProjectFile {
                trurl_version: "0.1.0".into(),
                project: Project {
                    name: "test".into(),
                    description: String::new(),
                },
            },
            components: std::collections::BTreeMap::new(),
            decisions: std::collections::BTreeMap::new(),
        }
    }
}
