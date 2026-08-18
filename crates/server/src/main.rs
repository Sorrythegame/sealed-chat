//! sealed-chat relay server entrypoint.

mod auth;
mod db;
mod error;
mod invites;
mod rate_limit;
mod routes;
mod state;
mod ws;

use axum::middleware;
use axum::routing::{get, post};
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sealed_server=debug,tower_http=info".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://sealed-chat.db".to_string());
    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-me".to_string());
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    let pool = db::init_pool(&database_url).await?;
    let state = AppState {
        db: pool,
        jwt_secret,
        connections: state::Connections::default(),
    };

    let auth_routes = Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .layer(middleware::from_fn_with_state(
            rate_limit::AuthRateLimiter::default(),
            rate_limit::limit_auth_requests,
        ));

    let app = Router::new()
        .route("/health", get(health))
        .merge(auth_routes)
        .route("/api/invites", post(invites::create_invite))
        .route("/api/users", get(routes::list_users))
        .route(
            "/api/users/by-name/{username}",
            get(routes::get_user_by_name),
        )
        .route("/api/users/{id}/devices", get(routes::get_user_devices))
        .route(
            "/api/conversations",
            post(routes::create_conversation).get(routes::list_conversations),
        )
        .route(
            "/api/conversations/{id}/messages",
            post(routes::send_message).get(routes::list_messages),
        )
        .route("/api/attachments", post(routes::upload_attachment))
        .route("/api/attachments/{id}", get(routes::download_attachment))
        .route(
            "/api/me",
            get(routes::get_profile).patch(routes::update_profile),
        )
        .route("/api/me/avatar", post(routes::update_avatar))
        .route("/ws", axum::routing::get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse()?;
    tracing::info!("sealed-server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
