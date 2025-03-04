use axum::{
    http::HeaderValue,
    routing::{get, post},
    Router,
};
use kanji_guesser::{
    api::{check_word, create_lobby, get_kanji, join_lobby},
    AppState,
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(AppState::new());

    // Configure CORS for development
    let cors = CorsLayer::new()
        // Allow requests from the development server
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/lobby/create", post(create_lobby))
        .route("/lobby/join/{lobby_id}", get(join_lobby))
        .route("/kanji/{lobby_id}", get(get_kanji))
        .route("/check_word/{lobby_id}", post(check_word))
        .with_state(app_state)
        .layer(cors) // Add CORS middleware
        .fallback_service(ServeDir::new("./static").append_index_html_on_directories(true));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server running on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
