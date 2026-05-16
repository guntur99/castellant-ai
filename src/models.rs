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
    pub role: String, // USER, SUPERADMIN
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
    pub plan_name: Option<String>,
    pub language: String,
    pub ai_chat_enabled: bool,
    pub ai_usage_count: i32,
    pub ai_custom_knowledge: Option<String>,
    pub ai_language: String,
    pub custom_song_url: Option<String>,
    pub background_video_url: Option<String>,
    pub hero_video_position: Option<i32>,
    pub stories_data: Option<serde_json::Value>,
    pub playlist: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

// Template Views (what the frontend sees)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Invitation {
    pub slug: String,
    pub template_name: String,
    pub couple_name_short: String,
    pub bride_name_short: String,
    pub groom_name_short: String,
    pub bride: Person,
    pub groom: Person,
    pub event_date: String,
    pub ceremony: EventDetails,
    pub reception: EventDetails,
    pub quote: Quote,
    pub gallery_images: Vec<String>,
    pub gallery_videos: Vec<String>,
    pub gift_accounts: Vec<GiftAccount>,
    pub song_url: String,
    pub song_id: Option<Uuid>,
    pub plan_name: String,
    pub ai_chat_enabled: bool,
    pub ai_usage_count: i32,
    pub ai_custom_knowledge: String,
    pub ai_language: String,
    pub recipient_name: String,
    pub event_date_iso: String,
    pub rsvps: Vec<Rsvp>,
    pub custom_song_url: String,
    pub background_video_url: String,
    pub hero_video_position: i32,
    pub stories: Vec<Story>,
    pub playlist: Vec<String>,
    pub is_preview: bool,
}

impl Invitation {
    pub fn to_json_context(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
    
    pub fn total_rsvps(&self) -> usize {
        self.rsvps.len()
    }
    
    pub fn total_hadir(&self) -> usize {
        self.rsvps.iter().filter(|r| r.attendance.to_lowercase() == "hadir").count()
    }
    
    pub fn total_guest_count(&self) -> i32 {
        self.rsvps.iter().filter(|r| r.attendance.to_lowercase() == "hadir").map(|r| r.guests as i32).sum()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Person {
    pub name: String,
    pub full_name: String,
    pub father_name: String,
    pub mother_name: String,
    pub image_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EventDetails {
    pub enabled: bool,
    pub date: String,
    pub time: String,
    pub venue: String,
    pub address: String,
    pub maps_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Story {
    pub id: String,
    pub image_url: String,
    pub title: String,
    pub date: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Quote {
    pub text: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, Default)]
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
    pub invitation_slug: String,
    pub name: String,
    pub attendance: String,
    pub guests: u8,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Rsvp {
    pub id: Uuid,
    pub invitation_id: Uuid,
    pub name: String,
    pub attendance: String,
    pub guests: i32,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Rsvp {
    pub fn initial(&self) -> String {
        self.name.chars().next().unwrap_or('G').to_string().to_uppercase()
    }
    
    pub fn display_message(&self) -> &str {
        self.message.as_deref().unwrap_or("")
    }

    pub fn display_attendance(&self) -> &str {
        if self.attendance.to_lowercase() == "hadir" {
            "✅ Hadir"
        } else {
            "❌ Tidak Hadir"
        }
    }

    pub fn attendance_class(&self) -> &str {
        if self.attendance.to_lowercase() == "hadir" {
            "text-green"
        } else {
            "text-red"
        }
    }

    pub fn attendance_style(&self) -> &str {
        if self.attendance.to_lowercase() == "hadir" {
            "color:green;font-weight:600"
        } else {
            "color:red;font-weight:600"
        }
    }

    pub fn is_attending_akad(&self) -> bool {
        self.attendance.to_lowercase() == "hadir"
    }

    pub fn is_attending_reception(&self) -> bool {
        self.attendance.to_lowercase() == "hadir"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct AiSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub chat_history: serde_json::Value,
    pub form_state: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, Default)]
pub struct Guest {
    pub id: Uuid,
    pub invitation_id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub template_override: Option<String>,
    pub slug: String,
    pub is_sent: bool,
    pub ai_language: String,
    pub song_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow, Default)]
pub struct GuestGroup {
    pub id: Uuid,
    pub invitation_id: Uuid,
    pub name: String,
    pub template_name: String,
    pub ai_language: String,
    pub song_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Booking {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub invitation_id: Uuid,
    pub target_plan: String,
    pub amount: i32,
    pub status: String, // PENDING, SUCCESS, FAILED
    pub invoice_id: String,
    pub payment_link: Option<String>,
    pub voucher_code: Option<String>,
    pub discount_amount: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Voucher {
    pub id: Uuid,
    pub code: String,
    pub discount_percent: i32,
    pub valid_until: Option<DateTime<Utc>>,
    pub usage_limit: Option<i32>,
    pub usage_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct InvitationTemplate {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[sqlx(rename = "description")]
    pub desc: String,
    pub category: String,
    pub preview_img: String,
    pub status: String,
    pub is_featured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
