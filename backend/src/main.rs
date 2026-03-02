use axum::{
    routing::{get, post},
    Router,
};
use moji::{
    api::ws_handler,
    db::init_db_pool,
    state::AppState,
};
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    wait_for_db(&database_url).await?;
    let db_pool = init_db_pool(&database_url).await?;

    let app_state = Arc::new(AppState::create_with_db(db_pool).await?);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let frontend_dir = if env::var("PRODUCTION").is_ok() {
        // In production (Docker), the frontend is in /usr/local/dist
        PathBuf::from("/usr/local/dist")
    } else {
        // In development, relative to the backend directory
        PathBuf::from("../frontend/dist")
    };

    tracing::info!("Serving frontend from: {:?}", frontend_dir);
    let index_path = frontend_dir.join("index.html");

    let api_context: Arc<dyn shared::ApiContext> = app_state.clone();
    let api_context_post = api_context.clone();
    let api_context_get = api_context.clone();

    let app = Router::new()
        .route("/ws/{lobby_id}/{player_id}", get(ws_handler))
        .route("/api/{*fn_name}", post(move |req: axum::extract::Request| {
            let ctx = api_context_post.clone();
            leptos_axum::handle_server_fns_with_context(
                move || {
                    leptos::context::provide_context(ctx.clone());
                },
                req,
            )
        }))
        .route("/api/{*fn_name}", get(move |req: axum::extract::Request| {
            let ctx = api_context_get.clone();
            leptos_axum::handle_server_fns_with_context(
                move || {
                    leptos::context::provide_context(ctx.clone());
                },
                req,
            )
        }))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        )
        .fallback_service(
            ServeDir::new(&frontend_dir).not_found_service(ServeFile::new(&index_path)),
        );

    // Get host and port from environment variables or use defaults
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;

    // Parse the host string to determine the bind address
    let addr = if host == "0.0.0.0" {
        // For Docker, bind to all interfaces
        SocketAddr::from(([0, 0, 0, 0], port))
    } else if host == "127.0.0.1" {
        // For local development
        SocketAddr::from(([127, 0, 0, 1], port))
    } else {
        // Try to parse as a general address
        format!("{}:{}", host, port).parse::<SocketAddr>()?
    };

    // Start the server
    tracing::info!("Server running on {}", addr);
    tracing::info!("Frontend available at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn wait_for_db(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::Connection;
    use std::time::Duration;
    use tokio::time::sleep;

    let mut retries = 5;
    let mut wait_time = Duration::from_secs(1);

    while retries > 0 {
        match sqlx::postgres::PgConnection::connect(database_url).await {
            Ok(_) => {
                tracing::info!("Successfully connected to database");
                return Ok(());
            }
            Err(e) => {
                retries -= 1;
                if retries == 0 {
                    return Err(Box::new(e));
                }
                tracing::warn!(
                    "Failed to connect to database, retrying in {:?}... ({} retries left)",
                    wait_time,
                    retries
                );
                sleep(wait_time).await;
                wait_time *= 2; // Exponential backoff
            }
        }
    }

    Ok(())
}
