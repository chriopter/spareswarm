mod api;
mod auth;
mod catalog;
mod db;
use api::*;
use axum::{Router, routing::get};

use std::{
    env,
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("SPARESWARM_TOKEN").expect("Set SPARESWARM_TOKEN to a strong API token");
    assert!(
        token.len() >= 24,
        "SPARESWARM_TOKEN must contain at least 24 characters"
    );
    let mut db = db::open(&env::var("DATABASE_PATH").unwrap_or("spareswarm.db".into()))?;
    catalog::sync(
        &mut db,
        std::path::Path::new(&env::var("PROJECTS_DIR").unwrap_or("projects".into())),
    )?;
    let state = Arc::new(Store {
        db: Mutex::new(db),
        admin_hash: auth::hash(&token),
    });
    let app = Router::new()
        .route("/health", get(health))
        .route("/version", get(|| async { ([(axum::http::header::CACHE_CONTROL, "no-store")], axum::Json(serde_json::json!({"commit": option_env!("BUILD_COMMIT").unwrap_or("development")}))) }))
        .route("/api/projects", get(projects))
        .route("/api/home", get(home))
        .route("/api/activity", get(activity))
        .route("/api/tasks", get(tasks).post(create))
        .route("/api/tasks/{id}", get(detail).patch(update))
        .route("/api/tasks/{id}/comments", axum::routing::post(comment))
        .route("/api/participants", axum::routing::post(auth::register))
        .route("/api/me", get(auth::me).patch(auth::edit_profile))
        .route("/api/tokens", axum::routing::post(auth::new_token))
        .route("/api/tokens/{id}", axum::routing::delete(auth::revoke))
        .route("/agent-instructions", get(|| async { include_str!("../../PROMPT.md") }))
        .route_service("/tasks/{id}", tower_http::services::ServeFile::new(std::path::Path::new(&env::var("WEB_DIR").unwrap_or("web".into())).join("task.html")))
        .fallback_service(ServeDir::new(env::var("WEB_DIR").unwrap_or("web".into())))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024))
        .layer(axum::middleware::from_fn(|request: axum::extract::Request, next: axum::middleware::Next| async move {
            let mut response = next.run(request).await;
            for (name, value) in [
                ("x-content-type-options", "nosniff"),
                ("referrer-policy", "no-referrer"),
                ("x-frame-options", "DENY"),
                ("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"),
            ] { response.headers_mut().insert(axum::http::HeaderName::from_static(name), axum::http::HeaderValue::from_static(value)); }
            response
        }))
        .with_state(state);
    let addr = env::var("LISTEN_ADDR").unwrap_or("127.0.0.1:8080".into());
    println!("SpareSwarm listening on {addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app)
        .with_graceful_shutdown(async {
            #[cfg(unix)]
            {
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("SIGTERM handler");
                tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
            }
            #[cfg(not(unix))]
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
