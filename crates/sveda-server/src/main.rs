use sveda_server::{app, AppState, Config};

#[tokio::main]
async fn main() {
    sveda_server::load_runtime_env();
    let bind = std::env::var("SVEDA_BIND").unwrap_or_else(|_| "0.0.0.0:8787".to_string());
    let state = AppState::live(Config::from_env()).await;
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|error| panic!("bind {bind}: {error}"));
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("sveda-server");
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = terminate.recv() => {}
        }
        return;
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}
