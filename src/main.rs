mod handlers;
mod models;
mod mailer;
mod filters;

use axum::{
    routing::{get, post},
    Router,
    extract::FromRef,
};
use axum::extract::DefaultBodyLimit;
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
    pub sumopod_api_key: String,
    pub sumopod_base_url: String,
    pub sumopod_model: String,
    pub s3_client: aws_sdk_s3::Client,
    pub s3_bucket: String,
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

    // DB Pool with robust retry logic to handle Railway proxy connection resets on quick restarts
    let mut db_pool = None;
    for attempt in 1..=3 {
        match PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await
        {
            Ok(pool) => {
                db_pool = Some(pool);
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    panic!("Failed to connect to Postgres after 3 attempts: {:?}", e);
                }
                tracing::warn!("Failed to connect to Postgres (attempt {}/3): {}. Retrying in 1.5 seconds...", attempt, e);
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(1.5)).await;
            }
        }
    }
    let db_pool = db_pool.unwrap();

    // Set timezone to Jakarta for all future sessions
    let _ = sqlx::query("SET timezone TO 'Asia/Jakarta'")
        .execute(&db_pool)
        .await;

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

    let sumopod_api_key = env::var("SUMOPOD_API_KEY").unwrap_or_default();
    let sumopod_base_url = env::var("SUMOPOD_BASE_URL").unwrap_or_else(|_| "https://ai.sumopod.com/v1/chat/completions".to_string());
    let sumopod_model = env::var("SUMOPOD_MODEL").unwrap_or_else(|_| "gemini/gemini-2.5-flash-lite".to_string());

    // S3 Config
    let s3_endpoint = env::var("S3_ENDPOINT").unwrap_or_default();
    let s3_region = env::var("S3_REGION").unwrap_or_else(|_| "auto".to_string());
    let s3_bucket = env::var("S3_BUCKET").unwrap_or_default();
    let s3_access_key = env::var("S3_ACCESS_KEY_ID").unwrap_or_default();
    let s3_secret_key = env::var("S3_SECRET_ACCESS_KEY").unwrap_or_default();

    let credentials = aws_sdk_s3::config::Credentials::new(
        s3_access_key,
        s3_secret_key,
        None,
        None,
        "static",
    );

    let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(credentials)
        .region(aws_config::Region::new(s3_region))
        .endpoint_url(s3_endpoint)
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&s3_config);

    // Cookie Key (Persistent across restarts)
    let session_secret = env::var("SESSION_SECRET").unwrap_or_else(|_| {
        "temporary_dev_key_that_is_at_least_64_bytes_long_so_axum_extra_x64".to_string()
    });
    let cookie_key = Key::from(session_secret.as_bytes());

    let state = AppState {
        db: db_pool,
        redis: redis_pool,
        oauth: oauth_client,
        cookie_key,
        http_client: reqwest::Client::new(),
        is_dev,
        mayar_api_key,
        mayar_base_url,
        sumopod_api_key,
        sumopod_base_url,
        sumopod_model,
        s3_client,
        s3_bucket,
    };

    // build our application with a route
    let app = Router::new()
        .route("/", get(handlers::home))
        .route("/dashboard", get(handlers::dashboard))
        .route("/profile", get(handlers::profile))
        .route("/settings", get(handlers::settings))
        .route("/invitation/{slug}", get(handlers::invitation_detail))
        .route("/api/rsvp", post(handlers::rsvp))
        .route("/auth/google", get(handlers::google_login))
        .route("/auth/google/callback", get(handlers::google_callback))
        .route("/auth/mock", get(handlers::mock_login))
        .route("/auth/logout", get(handlers::logout))
        .route("/create", get(handlers::create_invitation_page))
        .route("/templates", get(handlers::templates_list))
        .route("/admin/revenue", get(handlers::admin_revenue))
        .route("/admin/templates", get(handlers::admin_templates))
        .route("/admin/templates/new", get(handlers::admin_templates_new))
        .route("/admin/templates/create", post(handlers::admin_templates_create))
        .route("/admin/templates/{id}/edit", get(handlers::admin_templates_edit))
        .route("/admin/templates/{id}/update", post(handlers::admin_templates_update))
        .route("/admin/templates/{id}/toggle-status", post(handlers::admin_templates_toggle_status))
        .route("/admin/templates/{id}/toggle-featured", post(handlers::admin_templates_toggle_featured))
        .route("/admin/templates/{id}/delete", post(handlers::admin_templates_delete))
        .route("/api/invitation", post(handlers::create_invitation))
        .route("/api/preview", post(handlers::preview))
        .route("/api/ai/generate-text", post(handlers::ai_generate_text))
        .route("/invitation/{slug}/ai-chat", post(handlers::ai_guest_chat))
        .route("/api/ai/parse-form", post(handlers::ai_parse_form))
        .route("/api/ai/session/{id}", get(handlers::get_ai_session))
        .route("/invitation/{slug}/manage", get(handlers::manage_invitation).post(handlers::update_invitation))
        .route("/invitation/{slug}/delete", post(handlers::delete_invitation))
        .route("/invitation/{slug}/update-theme", post(handlers::update_theme))
        .route("/invitation/{slug}/guests", post(handlers::add_guest))
        .route("/invitation/{slug}/guests/{guest_id}/update", post(handlers::update_guest))
        .route("/invitation/{slug}/guests/{guest_id}/delete", post(handlers::delete_guest))
        .route("/invitation/{slug}/guests/{guest_id}/update-template", post(handlers::update_guest_template))
        .route("/invitation/{slug}/rsvp/{rsvp_id}/delete", post(handlers::delete_rsvp))
        .route("/invitation/{slug}/groups", post(handlers::add_group))
        .route("/invitation/{slug}/groups/{group_id}/update", post(handlers::update_group))
        .route("/invitation/{slug}/groups/{group_id}/delete", post(handlers::delete_group))
        .route("/api/payment/create-upgrade/{slug}", post(handlers::create_upgrade_payment))
        .route("/api/payment/webhook", post(handlers::mayar_webhook))
        .route("/api/test-email", get(handlers::test_email))
        .route("/api/check-slug/{slug}", get(handlers::check_slug))
        .route("/receipt/{invoice_id}", get(handlers::receipt_detail))
        .route("/sitemap.xml", get(handlers::sitemap))
        .route("/uploads/{*key}", get(handlers::serve_upload))
        .route("/robots.txt", get(|| async { 
            tokio::fs::read_to_string("static/robots.txt").await.unwrap_or_else(|_| "".to_string()) 
        }))
        .with_state(state)
        .nest_service("/static", ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024)) // 20MB limit for photo uploads
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
