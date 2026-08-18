//! WebSocket relay: pushes new (ciphertext) messages to online users.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;

use crate::auth::Claims;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user_id = match verify_token(&state, &query.token) {
        Ok(id) => id,
        Err(_) => {
            return (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

fn verify_token(state: &AppState, token: &str) -> Result<i64, ()> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims.sub)
    .map_err(|_| ())
}

async fn handle_socket(mut socket: WebSocket, state: AppState, user_id: i64) {
    let mut rx = state.connections.register(user_id);
    tracing::debug!("user {user_id} connected");

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
            outgoing = rx.recv() => {
                match outgoing {
                    Some(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    state.connections.unregister(user_id);
    tracing::debug!("user {user_id} disconnected");
}
