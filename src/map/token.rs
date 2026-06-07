//! CSPRNG token generation and request authentication.
//!
//! The map server is localhost-only. A 256-bit random token in the URL
//! prevents other local processes from accessing the API. Every HTTP
//! request must carry `Authorization: Bearer <token>`. WebSocket
//! upgrade passes the token as a query parameter (`/ws?token=...`)
//! because browser WebSocket APIs cannot set custom headers.

use std::fmt::Write;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

/// Generate a 256-bit random token from the OS CSPRNG, hex-encoded to
/// 64 lowercase characters.
pub(crate) fn generate() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut hex = String::with_capacity(64);
    for b in &bytes {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Axum middleware: reject requests without a valid `Authorization: Bearer`
/// token. Returns `401 Unauthorized` with an empty body on failure.
pub(crate) async fn require_bearer(
    State(expected): State<Arc<str>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let valid = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|t| constant_time_eq(t.as_bytes(), expected.as_bytes()));

    if valid {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Constant-time byte comparison. Prevents timing side-channels on the
/// token value. The threat model (localhost-only, token in URL) makes
/// this defence-in-depth rather than critical, but it costs nothing.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_64_hex_chars() {
        let token = generate();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_is_unique() {
        let a = generate();
        let b = generate();
        assert_ne!(a, b);
    }

    #[test]
    fn constant_time_eq_equal() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn constant_time_eq_unequal_same_len() {
        assert!(!constant_time_eq(b"abc", b"abd"));
    }

    #[test]
    fn constant_time_eq_different_len() {
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"ab", b"abc"));
    }
}
