mod handlers;
mod models;

use axum::{
    routing::{get, post},
    Router,
    extract::FromRef,
};
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};
use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use std::env;
use oauth2::{basic::BasicClient, EndpointSet, EndpointNotSet};
use axum_extra::extract::cookie::Key;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: deadpool_redis::Pool,
    pub oauth: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    pub cookie_key: Key,
    pub http_client: reqwest::Client,
    pub is_dev: bool,
    pub mayar_api_key: String,
    pub mayar_base_url: String,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
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

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

    // Redis Pool
    let cfg = deadpool_redis::Config::from_url(redis_url);
    let redis_pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    // OAuth2 Client
    let client_id = env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set");
    let client_secret = env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET must be set");
    let redirect_url = env::var("GOOGLE_REDIRECT_URL").expect("GOOGLE_REDIRECT_URL must be set");
    let auth_url = "https://accounts.google.com/o/oauth2/v2/auth".to_string();
    let token_url = "https://www.googleapis.com/oauth2/v4/token".to_string();

    let oauth_client = BasicClient::new(oauth2::ClientId::new(client_id))
        .set_client_secret(oauth2::ClientSecret::new(client_secret))
        .set_auth_uri(oauth2::AuthUrl::new(auth_url).unwrap())
        .set_token_uri(oauth2::TokenUrl::new(token_url).unwrap())
        .set_redirect_uri(oauth2::RedirectUrl::new(redirect_url).unwrap());

    let mode = env::var("MODE").unwrap_or_else(|_| "PROD".to_string());
    let is_dev = mode == "DEV";
    let mayar_api_key = env::var("MAYAR_API_KEY").unwrap_or_default();
    let mayar_base_url = env::var("MAYAR_BASE_URL").unwrap_or_else(|_| "https://api.mayar.club/hl/v1/invoice/create".to_string());

    let state = AppState {
        db: db_pool,
        redis: redis_pool,
        oauth: oauth_client,
        cookie_key: Key::generate(),
        http_client: reqwest::Client::new(),
        is_dev,
        mayar_api_key,
        mayar_base_url,
    };

    // build our application with a route
    let app = Router::new()
        .route("/", get(handlers::home))
        .route("/invitation/{slug}", get(handlers::invitation_detail))
        .route("/api/rsvp", post(handlers::rsvp))
        .route("/auth/google", get(handlers::google_login))
        .route("/auth/google/callback", get(handlers::google_callback))
        .route("/auth/mock", get(handlers::mock_login))
        .route("/auth/logout", get(handlers::logout))
        .route("/create", get(handlers::create_invitation_page))
        .route("/templates", get(handlers::templates_list))
        .route("/api/invitation", post(handlers::create_invitation))
        .route("/api/preview", post(handlers::preview))
        .route("/sitemap.xml", get(handlers::sitemap))
        .route("/robots.txt", get(|| async { 
            tokio::fs::read_to_string("static/robots.txt").await.unwrap_or_else(|_| "".to_string()) 
        }))
        .with_state(state)
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http());

    // run our app with hyper
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("\n🚀 Server is running on http://localhost:{}", port);
    tracing::info!("listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
