mod api;
mod traceroute;

use std::env;

#[tokio::main]
async fn main() {
    let app = api::router();

    let port = env::var("PORT");
    let port = match port {
        Ok(value) => value,
        Err(_) => "3000".to_string(),
    };
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("failed to bind to port 3000");

    println!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .await
        .expect("server error");
}
