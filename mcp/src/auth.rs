use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

/// Axum middleware: reject any request whose `Authorization` header is not
/// exactly `Bearer <token>`. Token comparison is constant-time.
pub async fn require_bearer_token(
    State(token): State<Arc<String>>,
    req: Request,
    next: Next,
) -> Response {
    let ok = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        // `ct_eq` is constant-time only for equal-length inputs; subtle
        // short-circuits (non-constant-time) on a length mismatch. This
        // leaks token *length* via timing, not its contents — a known,
        // accepted trade-off here.
        .is_some_and(|t| t.as_bytes().ct_eq(token.as_bytes()).into());
    if ok {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
