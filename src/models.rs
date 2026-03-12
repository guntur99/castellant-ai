use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Database Entities
#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub google_id: Option<String>,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct InvitationRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub slug: String,
    pub couple_name_short: String,
    pub template_name: String,
    pub event_date: String,
    pub song_id: Option<Uuid>,
    pub bride_data: serde_json::Value,
    pub groom_data: serde_json::Value,
    pub ceremony_data: serde_json::Value,
    pub reception_data: serde_json::Value,
    pub quote_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// Template Views (what the frontend sees)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invitation {
    pub slug: String,
    pub couple_name_short: String,
    pub bride: Person,
    pub groom: Person,
    pub event_date: String,
    pub ceremony: EventDetails,
    pub reception: EventDetails,
    pub quote: Quote,
    pub gallery_images: Vec<String>,
    pub gift_accounts: Vec<GiftAccount>,
    pub song_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Person {
    pub name: String,
    pub full_name: String,
    pub father_name: String,
    pub mother_name: String,
    pub image_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventDetails {
    pub date: String,
    pub time: String,
    pub venue: String,
    pub address: String,
    pub maps_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Quote {
    pub text: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct GiftAccount {
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Song {
    pub id: Uuid,
    pub title: String,
    pub artist: String,
    pub file_path: String,
    pub audio_data: Option<Vec<u8>>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct RsvpForm {
    pub name: String,
    pub attendance: String,
    pub guests: u8,
    pub message: String,
}
