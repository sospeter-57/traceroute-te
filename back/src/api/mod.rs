pub fn router() -> axum::Router {
    axum::Router::new().route("/run", axum::routing::get(|| async { "Hello"}))
}

