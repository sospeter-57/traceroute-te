use crate::traceroute::ws;

/// Routes the WebSocket endpoint that streams traceroute hops. There's no
/// HTTP API yet — the only client is the frontend, which speaks WS.
pub fn router() -> axum::Router {
    axum::Router::new().route("/ws/traceroute", axum::routing::get(ws::handler))
}

