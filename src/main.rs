use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod db;
mod indexer;
mod types;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub ready: Arc<AtomicBool>,
}

#[tokio::main]
async fn main() {
    eprintln!("=== molt-marketplace-backend starting ===");

    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "molt_marketplace_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create pool lazily — no actual connection yet, server can start immediately
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect_lazy(&database_url)
        .expect("Failed to create connection pool");

    tracing::info!("Connection pool created (lazy, no connection yet)");

    let ready = Arc::new(AtomicBool::new(false));

    // Build application state
    let state = AppState {
        pool: pool.clone(),
        ready: ready.clone(),
    };

    // Set up CORS (allow all origins for development)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api", api::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server FIRST (Railway injects PORT env var)
    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to port");

    tracing::info!("Server listening on {addr}");

    // Run migrations + indexer in background so the server accepts connections immediately
    let enable_indexer = std::env::var("ENABLE_INDEXER").unwrap_or_default() == "true";
    let bg_pool = pool.clone();
    tokio::spawn(async move {
        // Retry migrations up to 5 times with backoff (handles connection pool contention during deploys)
        let max_retries = 5;
        for attempt in 1..=max_retries {
            tracing::info!("Running migrations (attempt {}/{})", attempt, max_retries);
            match sqlx::migrate!("./migrations").run(&bg_pool).await {
                Ok(_) => {
                    tracing::info!("Migrations applied successfully");
                    break;
                }
                Err(e) if attempt < max_retries => {
                    tracing::warn!("Migration attempt {} failed: {:?} — retrying in {}s", attempt, e, attempt * 5);
                    tokio::time::sleep(std::time::Duration::from_secs(attempt as u64 * 5)).await;
                }
                Err(e) => {
                    tracing::error!("Migration failed after {} attempts: {:?}", max_retries, e);
                    return;
                }
            }
        }

        // Backfill block_timestamp for existing rows (one-time, idempotent)
        indexer::backfill::backfill_block_timestamps(&bg_pool).await;

        ready.store(true, Ordering::Release);
        tracing::info!("Database ready — accepting API requests");

        // Start indexer after migrations are done
        if enable_indexer {
            tracing::info!("Indexer background task started");
            indexer::run_indexer(bg_pool).await;
        } else {
            tracing::info!("Indexer disabled (set ENABLE_INDEXER=true to enable)");
        }
    });

    // Run the API server (blocks until shutdown)
    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    if state.ready.load(Ordering::Acquire) {
        (StatusCode::OK, "OK")
    } else {
        // Return 200 so Railway knows the container is alive, but indicate not fully ready
        (StatusCode::OK, "starting")
    }
}
