use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn require_auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let expected = std::env::var("BEARER_TOKEN").unwrap_or_default();

    let from_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("")
        .to_string();

    // EventSource (SSE) can't set headers, so also accept token as query param
    let from_query = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&').find_map(|kv| {
                let mut parts = kv.splitn(2, '=');
                if parts.next() == Some("token") { parts.next().map(str::to_string) } else { None }
            })
        })
        .unwrap_or_default();

    let provided = if !from_header.is_empty() { from_header } else { from_query };

    if provided != expected || expected.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
