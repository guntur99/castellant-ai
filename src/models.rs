use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GiftAccount {
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
}

#[derive(Debug, Deserialize)]
pub struct RsvpForm {
    pub name: String,
    pub attendance: String,
    pub guests: u8,
    pub message: String,
}
