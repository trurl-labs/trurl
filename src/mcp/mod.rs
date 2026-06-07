mod context;
mod prompts;
mod protocol;
mod tools;
mod write;

use std::io::{self};
use std::path::Path;

use serde_json::Value;

use crate::Result;
use crate::store::{ProjectState, Store};

use protocol::{INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, Request, Response};

const PROTOCOL_VERSION: &str = "2024-11-05";

// ── Public entry point ────────────────────────────────────────────────────

pub(crate) fn run_server(store_root: &Path) -> Result<()> {
    let store = Store::at(store_root.to_path_buf());
    let mut state = store.load_state()?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut initialized = false;

    eprintln!("trurl: MCP server ready");

    loop {
        match protocol::read_message(&mut reader) {
            Ok(Some(request)) => {
                if let Some(response) = handle(&store, &mut state, request, &mut initialized) {
                    if let Err(e) = protocol::write_response(&mut writer, &response) {
                        eprintln!("trurl: stdout write error: {e}");
                        break;
                    }
                }
            }
            Ok(None) => break, // EOF — clean shutdown
            Err(e) => {
                let response = Response::error(Value::Null, PARSE_ERROR, e.to_string());
                if let Err(we) = protocol::write_response(&mut writer, &response) {
                    eprintln!("trurl: stdout write error: {we}");
                    break;
                }
            }
        }
    }

    eprintln!("trurl: MCP server stopped");
    Ok(())
}

// ── Request dispatch ──────────────────────────────────────────────────────

fn handle(
    store: &Store,
    state: &mut ProjectState,
    request: Request,
    initialized: &mut bool,
) -> Option<Response> {
    // Notifications never receive a response.
    if request.is_notification() {
        // notifications/initialized: acknowledged, no state change needed
        // (initialized flag was set when we responded to the initialize request).
        return None;
    }

    let id = request.id.unwrap_or(Value::Null);

    // JSON-RPC 2.0 §4.1: jsonrpc MUST be exactly "2.0".
    if request.jsonrpc != "2.0" {
        return Some(Response::error(
            id,
            INVALID_REQUEST,
            format!(
                "invalid jsonrpc version: expected \"2.0\", got {:?}",
                request.jsonrpc
            ),
        ));
    }

    let result = match request.method.as_str() {
        "initialize" => {
            *initialized = true;
            handle_initialize()
        }
        "ping" => Ok(serde_json::json!({})),
        // Gate: tool operations require initialization.
        "tools/list" if *initialized => Ok(tools::tool_list()),
        "tools/call" if *initialized => handle_tools_call(store, state, &request.params),
        "tools/list" | "tools/call" => Err((
            INVALID_REQUEST,
            "server not initialized — send initialize first".into(),
        )),
        _ => Err((
            METHOD_NOT_FOUND,
            format!("unknown method: {}", request.method),
        )),
    };

    Some(match result {
        Ok(value) => Response::success(id, value),
        Err((code, msg)) => Response::error(id, code, msg),
    })
}

fn handle_initialize() -> std::result::Result<Value, (i32, String)> {
    Ok(serde_json::json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "trurl",
            "version": env!("CARGO_PKG_VERSION"),
        }
    }))
}

fn handle_tools_call(
    store: &Store,
    state: &mut ProjectState,
    params: &Option<Value>,
) -> std::result::Result<Value, (i32, String)> {
    let params = params
        .as_ref()
        .ok_or_else(|| (INVALID_PARAMS, "missing params in tools/call".to_string()))?;

    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| (INVALID_PARAMS, "missing or invalid tool name".to_string()))?;

    let default_args = serde_json::json!({});
    let arguments = params.get("arguments").unwrap_or(&default_args);

    Ok(tools::call_tool(store, state, name, arguments))
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_request(id: Option<Value>, method: &str, params: Option<Value>) -> Request {
        Request {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }

    fn empty_state() -> ProjectState {
        use crate::store::schema::*;
        use chrono::Utc;
        ProjectState::new(
            ProjectFile {
                trurl_version: "0.2.0".into(),
                project: Project {
                    name: "test".into(),
                    description: String::new(),
                },
            },
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            GraphIndex {
                version: 1,
                rebuilt: Utc::now(),
                nodes: vec![],
                edges: vec![],
            },
        )
    }

    fn empty_store() -> (tempfile::TempDir, Store) {
        let tmp = tempfile::TempDir::new().unwrap();
        crate::commands::init(tmp.path()).unwrap();
        let store = Store::discover(tmp.path()).unwrap();
        (tmp, store)
    }

    #[test]
    fn notification_returns_none() {
        let (_tmp, store) = empty_store();
        let mut state = store.load_state().unwrap();
        let mut initialized = true;
        let req = make_request(None, "notifications/initialized", None);
        assert!(handle(&store, &mut state, req, &mut initialized).is_none());
    }

    #[test]
    fn initialize_returns_capabilities() {
        let (_tmp, store) = empty_store();
        let mut state = store.load_state().unwrap();
        let mut initialized = false;
        let req = make_request(Some(json!(1)), "initialize", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        assert!(initialized);
        let json = serde_json::to_value(&resp).unwrap();
        let result = &json["result"];
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["serverInfo"]["name"], "trurl");
    }

    #[test]
    fn ping_returns_empty_object() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = true;
        let req = make_request(Some(json!(2)), "ping", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["result"], json!({}));
    }

    #[test]
    fn unknown_method_returns_error() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = true;
        let req = make_request(Some(json!(3)), "bogus/method", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn tools_call_missing_params_returns_error() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = true;
        let req = make_request(Some(json!(4)), "tools/call", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], INVALID_PARAMS);
    }

    #[test]
    fn tools_call_missing_name_returns_error() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = true;
        let req = make_request(Some(json!(5)), "tools/call", Some(json!({"arguments": {}})));
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], INVALID_PARAMS);
    }

    #[test]
    fn tools_list_returns_all_tools() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = true;
        let req = make_request(Some(json!(6)), "tools/list", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        let tools = json["result"]["tools"].as_array().unwrap();
        assert!(tools.len() >= 3);
    }

    #[test]
    fn tools_call_before_initialize_rejected() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = false;
        let req = make_request(
            Some(json!(7)),
            "tools/call",
            Some(json!({"name": "get_architecture", "arguments": {}})),
        );
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], INVALID_REQUEST);
        assert!(
            json["error"]["message"]
                .as_str()
                .unwrap()
                .contains("not initialized")
        );
    }

    #[test]
    fn tools_list_before_initialize_rejected() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = false;
        let req = make_request(Some(json!(8)), "tools/list", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], INVALID_REQUEST);
    }

    #[test]
    fn invalid_jsonrpc_version_rejected() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = true;
        let req = Request {
            jsonrpc: "1.0".into(),
            id: Some(json!(9)),
            method: "ping".into(),
            params: None,
        };
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], INVALID_REQUEST);
    }

    #[test]
    fn ping_allowed_before_initialize() {
        let mut state = empty_state();
        let store = Store::at("/dev/null/.trurl".into());
        let mut initialized = false;
        let req = make_request(Some(json!(10)), "ping", None);
        let resp = handle(&store, &mut state, req, &mut initialized).unwrap();
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["result"], json!({}));
    }
}
