use veda_server::{app, AppState, Config};

#[tokio::main]
async fn main() {
    let bind = std::env::var("VEDA_BIND").unwrap_or_else(|_| "0.0.0.0:8787".to_string());
    let state = AppState::live(Config::from_env());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|error| panic!("bind {bind}: {error}"));
    axum::serve(listener, app(state))
        .await
        .expect("veda-server");
}
