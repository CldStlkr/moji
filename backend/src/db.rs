use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{sync::Arc, time::Duration};

/// Database connection pool type
pub type DbPool = Pool<Postgres>;

/// Initialize a connection pool to the PostgreSQL database
pub async fn init_db_pool(database_url: &str) -> Result<Arc<DbPool>, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await?;

    // Run migrations to ensure the database is up-to-date
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Log successful connection
    tracing::info!("Connected to PostgreSQL database");

    Ok(Arc::new(pool))
}

/// Health check function to verify database connectivity
pub async fn check_database_connection(pool: &DbPool) -> Result<(), sqlx::Error> {
    // Simple query to verify the connection is working
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
