use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use subtle::ConstantTimeEq;

pub async fn require_token(
    State(expected): State<Arc<String>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let provided = extract_token(&req);
    if tokens_match(expected.as_bytes(), provided.as_deref().map(str::as_bytes)) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn extract_token(req: &Request) -> Option<String> {
    if let Some(val) = req.headers().get("X-Auth-Token") {
        return val.to_str().ok().map(str::to_owned);
    }
    let query = req.uri().query().unwrap_or("");
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("token=") {
            return Some(urlencoding_decode(value));
        }
    }
    None
}

fn urlencoding_decode(s: &str) -> String {
    // Minimal percent-decode for the token query param (tokens are hex strings in practice).
    // If the token contains no percent-encoding this is a no-op.
    percent_decode(s)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(b) = u8::from_str_radix(hex, 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn tokens_match(expected: &[u8], provided: Option<&[u8]>) -> bool {
    let Some(provided) = provided else {
        return false;
    };
    if expected.len() != provided.len() {
        return false;
    }
    expected.ct_eq(provided).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_tokens() {
        assert!(tokens_match(b"secret", Some(b"secret")));
    }

    #[test]
    fn mismatched_tokens() {
        assert!(!tokens_match(b"secret", Some(b"wrong__")));
    }

    #[test]
    fn length_mismatch_rejected() {
        assert!(!tokens_match(b"secret", Some(b"sec")));
    }

    #[test]
    fn none_rejected() {
        assert!(!tokens_match(b"secret", None));
    }

    #[test]
    fn extract_token_from_header() {
        let req = Request::builder()
            .header("X-Auth-Token", "mytoken")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_token(&req).as_deref(), Some("mytoken"));
    }

    #[test]
    fn extract_token_from_query_param() {
        let req = Request::builder()
            .uri("/trmnl?token=mytoken")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_token(&req).as_deref(), Some("mytoken"));
    }

    #[test]
    fn header_wins_over_query_param() {
        let req = Request::builder()
            .uri("/trmnl?token=query-token")
            .header("X-Auth-Token", "header-token")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(extract_token(&req).as_deref(), Some("header-token"));
    }

    #[test]
    fn no_token_returns_none() {
        let req = Request::builder()
            .uri("/trmnl")
            .body(axum::body::Body::empty())
            .unwrap();
        assert!(extract_token(&req).is_none());
    }
}
