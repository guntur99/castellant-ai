use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Adding AI Chat columns to invitations table...");

    sqlx::query("ALTER TABLE invitations ADD COLUMN IF NOT EXISTS ai_chat_enabled BOOLEAN DEFAULT FALSE")
        .execute(&pool)
        .await?;

    sqlx::query("ALTER TABLE invitations ADD COLUMN IF NOT EXISTS ai_usage_count INTEGER DEFAULT 0")
        .execute(&pool)
        .await?;

    println!("Success!");

    Ok(())
}
