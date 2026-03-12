use axum::{
    response::{Html, IntoResponse, Response},
    http::StatusCode,
    Form,
    extract::State,
};
use askama::Template;
use crate::models::{Invitation, Person, EventDetails, Quote, GiftAccount, RsvpForm};
use crate::AppState;

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub title: String,
}

#[derive(Template)]
#[template(path = "invitation/vintage.html")]
pub struct VintageTemplate {
    pub invitation: Invitation,
}

pub async fn home(State(_state): State<AppState>) -> impl IntoResponse {
    HtmlTemplate(HomeTemplate {
        title: "Castellant - Digital Invitation".to_string(),
    })
}

pub async fn invitation_detail(
    State(_state): State<AppState>,
    // In a real app, you'd fetch this from a DB based on the slug
    // axum::extract::Path(slug): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Sample data based on TikTok reference
    let invitation = Invitation {
        slug: "romeo-julia".to_string(),
        couple_name_short: "Romeo & Julia".to_string(),
        bride: Person {
            name: "Julia".to_string(),
            full_name: "Julia Capulet".to_string(),
            father_name: "Mr. Capulet".to_string(),
            mother_name: "Mrs. Capulet".to_string(),
            image_url: "/static/img/bride.jpg".to_string(),
        },
        groom: Person {
            name: "Romeo".to_string(),
            full_name: "Romeo Montague".to_string(),
            father_name: "Mr. Montague".to_string(),
            mother_name: "Mrs. Montague".to_string(),
            image_url: "/static/img/groom.jpg".to_string(),
        },
        event_date: "12 Desember 2026".to_string(),
        ceremony: EventDetails {
            date: "Sabtu, 12 Desember 2026".to_string(),
            time: "09:00 - 10:00 WIB".to_string(),
            venue: "Gereja Katedral".to_string(),
            address: "Jl. Katedral No.7, Jakarta Pusat".to_string(),
            maps_url: "https://maps.app.goo.gl/xxx".to_string(),
        },
        reception: EventDetails {
            date: "Sabtu, 12 Desember 2026".to_string(),
            time: "11:00 - 13:00 WIB".to_string(),
            venue: "The Glass House".to_string(),
            address: "Kawasan Menteng, Jakarta Pusat".to_string(),
            maps_url: "https://maps.app.goo.gl/yyy".to_string(),
        },
        quote: Quote {
            text: "Dan di antara tanda-tanda (kebesaran)-Nya ialah Dia menciptakan pasangan-pasangan untukmu dari jenismu sendiri, agar kamu cenderung dan merasa tenteram kepadanya, dan Dia menjadikan di antaramu rasa kasih dan sayang.".to_string(),
            source: "QS. Ar-Rum: 21".to_string(),
        },
        gallery_images: vec![
            "/static/img/gallery1.jpg".to_string(),
            "/static/img/gallery2.jpg".to_string(),
            "/static/img/gallery3.jpg".to_string(),
        ],
        gift_accounts: vec![
            GiftAccount {
                bank_name: "BCA".to_string(),
                account_number: "1234567890".to_string(),
                account_holder: "Julia Capulet".to_string(),
            },
        ],
    };

    HtmlTemplate(VintageTemplate { invitation })
}

pub async fn rsvp(
    State(_state): State<AppState>,
    Form(payload): Form<RsvpForm>
) -> impl IntoResponse {
    println!("RSVP received: {:?}", payload);
    Html(format!(
        r#"<div id="rsvp-response" class="animate__animated animate__fadeIn" style="background: #e8f5e9; padding: 1rem; border-radius: 10px; color: #2e7d32; text-align: center;">
            <p><strong>Terima kasih, {}!</strong></p>
            <p>Konfirmasi Anda telah kami terima.</p>
        </div>"#,
        payload.name
    ))
}
