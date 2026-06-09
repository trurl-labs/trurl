use std::io::{BufRead, Write};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct Request {
    /// Must be `"2.0"`.
    pub jsonrpc: String,

    /// Absent → notification (`None`).  Present (including explicit `null`) →
    /// request that expects a response (`Some`).
    /// Standard serde maps both absent *and* JSON `null` to `None` for
    /// `Option<Value>`.  The custom deserializer below preserves the
    /// distinction required by JSON-RPC 2.0 §4.1.
    #[serde(default, deserialize_with = "deserialize_optional_id")]
    pub id: Option<Value>,

    /// RPC method name (e.g. `"initialize"`, `"tools/call"`).
    pub method: String,

    /// Method parameters (may be absent).
    #[serde(default)]
    pub params: Option<Value>,
}

impl Request {
    /// A notification has no `id` and MUST NOT receive a response.
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// Deserialize a present JSON value (including `null`) as `Some(Value)`.
/// Paired with `#[serde(default)]`, absent fields become `None` while
/// explicit `null` becomes `Some(Value::Null)` — preserving the JSON-RPC 2.0
/// distinction between notifications (no `id` key) and requests with a
/// null identifier.
fn deserialize_optional_id<'de, D>(deserializer: D) -> std::result::Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

// ── Standard error codes (JSON-RPC 2.0 §5.1) ─────────────────────────────

pub(crate) const PARSE_ERROR: i32 = -32700;
pub(crate) const INVALID_REQUEST: i32 = -32600;
pub(crate) const METHOD_NOT_FOUND: i32 = -32601;
pub(crate) const INVALID_PARAMS: i32 = -32602;

const MAX_MESSAGE_BYTES: usize = 10 * 1024 * 1024;

// ── Stdio transport ───────────────────────────────────────────────────────

/// Read one JSON-RPC message from a buffered reader.
/// Skips blank lines. Returns `Ok(None)` on EOF (clean shutdown).
/// Returns `Err` on malformed JSON or messages exceeding the size limit.
pub(crate) fn read_message(reader: &mut impl BufRead) -> std::io::Result<Option<Request>> {
    loop {
        let line = match read_line_bounded(reader, MAX_MESSAGE_BYTES)? {
            Some(line) => line,
            None => return Ok(None), // EOF
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue; // skip blank lines between messages
        }
        return serde_json::from_str(trimmed)
            .map(Some)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
    }
}

/// Read a single newline-terminated line with bounded memory usage.
/// Reads incrementally from the buffered reader, returning `Err` if the
/// accumulated line exceeds `limit` bytes before a newline is found.
/// Returns `Ok(None)` on EOF with no data read.
fn read_line_bounded(reader: &mut impl BufRead, limit: usize) -> std::io::Result<Option<String>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                String::from_utf8(line)
                    .map(Some)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            };
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(pos) => {
                line.extend_from_slice(&available[..=pos]);
                let consumed = pos + 1;
                reader.consume(consumed);
                if line.len() > limit {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("message exceeds {limit} byte limit"),
                    ));
                }
                return String::from_utf8(line)
                    .map(Some)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e));
            }
            None => {
                let len = available.len();
                line.extend_from_slice(available);
                reader.consume(len);
                if line.len() > limit {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("message exceeds {limit} byte limit"),
                    ));
                }
            }
        }
    }
}

// ── Response writers ─────────────────────────────────────────────────────

/// Write a JSON-RPC 2.0 success response, serializing `result` directly
/// to the writer in a single pass.
///
/// The generic `R` allows callers to pass any `Serialize` type — a `Value`
/// for cold-path responses (initialize, ping, tools/list) or a typed struct
/// for hot-path tool results. Typed structs skip the intermediate `Value`
/// allocation that `serde_json::to_value` would introduce.
pub(crate) fn write_success<R: Serialize>(
    writer: &mut impl Write,
    id: &Value,
    result: &R,
) -> std::io::Result<()> {
    #[derive(Serialize)]
    struct SuccessEnvelope<'a, R: Serialize> {
        jsonrpc: &'static str,
        id: &'a Value,
        result: &'a R,
    }
    serde_json::to_writer(
        &mut *writer,
        &SuccessEnvelope {
            jsonrpc: "2.0",
            id,
            result,
        },
    )
    .map_err(std::io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// Write a JSON-RPC 2.0 error response.
///
/// Takes `message` by reference to avoid allocating a `String` for the
/// common case where the caller has a `&str` or `&format!(…)`.
pub(crate) fn write_error(
    writer: &mut impl Write,
    id: &Value,
    code: i32,
    message: &str,
) -> std::io::Result<()> {
    #[derive(Serialize)]
    struct ErrorEnvelope<'a> {
        jsonrpc: &'static str,
        id: &'a Value,
        error: RpcError<'a>,
    }
    #[derive(Serialize)]
    struct RpcError<'a> {
        code: i32,
        message: &'a str,
    }
    serde_json::to_writer(
        &mut *writer,
        &ErrorEnvelope {
            jsonrpc: "2.0",
            id,
            error: RpcError { code, message },
        },
    )
    .map_err(std::io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    // ── Request deserialization ──────────────────────────────────────────

    #[test]
    fn deserialize_request_with_id() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, Some(json!(1)));
        assert!(!req.is_notification());
    }

    #[test]
    fn deserialize_notification_no_id() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert!(req.is_notification());
    }

    #[test]
    fn deserialize_request_with_null_id() {
        let json = r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert!(!req.is_notification());
        assert_eq!(req.id, Some(Value::Null));
    }

    #[test]
    fn deserialize_request_with_params() {
        let json = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"get_context","arguments":{"component":"auth"}}}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/call");
        let params = req.params.unwrap();
        assert_eq!(params["name"], "get_context");
        assert_eq!(params["arguments"]["component"], "auth");
    }

    // ── write_success ───────────────────────────────────────────────────

    #[test]
    fn write_success_produces_valid_json_rpc() {
        let mut buf = Vec::new();
        write_success(&mut buf, &json!(1), &json!({"ok": true})).unwrap();
        let v: Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["ok"], true);
        assert!(v.get("error").is_none());
    }

    #[test]
    fn write_success_single_newline_terminated_line() {
        let mut buf = Vec::new();
        write_success(&mut buf, &json!(42), &json!("ok")).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output.lines().count(), 1);
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn write_success_serializes_typed_struct_directly() {
        #[derive(Serialize)]
        struct Custom {
            data: &'static str,
        }
        let mut buf = Vec::new();
        write_success(&mut buf, &json!(1), &Custom { data: "hello" }).unwrap();
        let v: Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["result"]["data"], "hello");
    }

    #[test]
    fn write_success_with_null_id() {
        let mut buf = Vec::new();
        write_success(&mut buf, &Value::Null, &json!(true)).unwrap();
        let v: Value = serde_json::from_slice(&buf).unwrap();
        assert!(v["id"].is_null());
        assert_eq!(v["result"], true);
    }

    // ── write_error ─────────────────────────────────────────────────────

    #[test]
    fn write_error_produces_valid_json_rpc() {
        let mut buf = Vec::new();
        write_error(&mut buf, &json!(2), METHOD_NOT_FOUND, "no such method").unwrap();
        let v: Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 2);
        assert_eq!(v["error"]["code"], METHOD_NOT_FOUND);
        assert_eq!(v["error"]["message"], "no such method");
        assert!(v.get("result").is_none());
    }

    #[test]
    fn write_error_single_newline_terminated_line() {
        let mut buf = Vec::new();
        write_error(&mut buf, &json!(1), PARSE_ERROR, "bad json").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output.lines().count(), 1);
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn write_error_with_null_id() {
        let mut buf = Vec::new();
        write_error(&mut buf, &Value::Null, PARSE_ERROR, "malformed").unwrap();
        let v: Value = serde_json::from_slice(&buf).unwrap();
        assert!(v["id"].is_null());
        assert_eq!(v["error"]["code"], PARSE_ERROR);
    }

    // ── read_message ────────────────────────────────────────────────────

    #[test]
    fn read_message_valid_json() {
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
        let mut reader = Cursor::new(input.as_slice());
        let req = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(req.method, "ping");
    }

    #[test]
    fn read_message_eof() {
        let mut reader = Cursor::new(b"".as_slice());
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn read_message_skips_blank_lines() {
        let input = b"\n\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
        let mut reader = Cursor::new(input.as_slice());
        let req = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(req.method, "ping");
    }

    #[test]
    fn read_message_rejects_invalid_json() {
        let input = b"not valid json\n";
        let mut reader = Cursor::new(input.as_slice());
        let err = read_message(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    // ── read_line_bounded ───────────────────────────────────────────────

    #[test]
    fn bounded_read_normal_line() {
        let input = b"hello world\n";
        let mut reader = Cursor::new(input.as_slice());
        let line = read_line_bounded(&mut reader, 1024).unwrap().unwrap();
        assert_eq!(line.trim(), "hello world");
    }

    #[test]
    fn bounded_read_eof_no_data() {
        let mut reader = Cursor::new(b"".as_slice());
        assert!(read_line_bounded(&mut reader, 1024).unwrap().is_none());
    }

    #[test]
    fn bounded_read_eof_with_partial_data() {
        let input = b"no newline";
        let mut reader = Cursor::new(input.as_slice());
        let line = read_line_bounded(&mut reader, 1024).unwrap().unwrap();
        assert_eq!(line, "no newline");
    }

    #[test]
    fn bounded_read_rejects_oversized_line() {
        let input = b"this line is way too long\n";
        let mut reader = Cursor::new(input.as_slice());
        let err = read_line_bounded(&mut reader, 10).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("byte limit"));
    }

    #[test]
    fn bounded_read_rejects_oversized_without_newline() {
        // Simulates a stream that keeps sending without a newline.
        let input = vec![b'x'; 2048];
        let mut reader = Cursor::new(input.as_slice());
        let err = read_line_bounded(&mut reader, 1024).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
