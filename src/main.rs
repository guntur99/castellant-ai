mod handlers;
mod models;

use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};
use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use std::env;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: deadpool_redis::Pool,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // initialize tracing
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");

    // DB Pool
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    // Redis Pool
    let cfg = deadpool_redis::Config::from_url(redis_url);
    let redis_pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    let state = AppState {
        db: db_pool,
        redis: redis_pool,
    };

    // build our application with a route
    let app = Router::new()
        .route("/", get(handlers::home))
        .route("/invitation/sample", get(handlers::invitation_detail))
        .route("/api/rsvp", post(handlers::rsvp))
        .with_state(state)
        // Serve static files from the "static" directory
        .fallback_service(ServeDir::new("static"))
        .layer(TraceLayer::new_for_http());

    // run our app with hyper
    // `axum::Server` has been replaced by `axum::serve` in newer versions
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
