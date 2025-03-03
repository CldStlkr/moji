use axum::{
    routing::{get, post},
    Router,
};
use kanji_guesser::{
    api::{check_word, create_lobby, get_kanji, join_lobby},
    AppState,
};
use std::sync::Arc;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/lobby/create", post(create_lobby))
        .route("/lobby/join/{lobby_id}", get(join_lobby))
        .route("/kanji/{lobby_id}", get(get_kanji))
        .route("/check_word/{lobby_id}", post(check_word))
        .with_state(app_state)
        .fallback_service(ServeDir::new("./static").append_index_html_on_directories(true));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server running on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
