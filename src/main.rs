use axum::{
    routing::{get, post},
    Router,
};
use kanji_guesser::{
    api::{check_word, create_lobby, generate_new_kanji, get_kanji, join_lobby},
    db::init_db_pool,
    AppState,
};
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file if present
    dotenv::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get database URL from environment variable
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Initialize database connection pool
    let db_pool = init_db_pool(&database_url).await?;

    // Initialize app state with database pool
    let app_state = Arc::new(AppState::new_with_db(db_pool).await?);

    // Configure CORS for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application router
    let app = Router::new()
        .route("/lobby/create", post(create_lobby))
        .route("/lobby/join/{lobby_id}", get(join_lobby))
        .route("/kanji/{lobby_id}", get(get_kanji))
        .route("/new_kanji/{lobby_id}", post(generate_new_kanji))
        .route("/check_word/{lobby_id}", post(check_word))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .fallback_service(ServeDir::new("./static").append_index_html_on_directories(true));

    // Get host and port from environment variables or use defaults
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    let addr = format!("{}:{}", host, port).parse::<SocketAddr>()?;

    // Start the server
    tracing::info!("Server running on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
