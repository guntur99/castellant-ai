use axum::{
    response::{Html, IntoResponse, Response, Redirect},
    http::StatusCode,
    Form,
    extract::{State, Path, Query, Multipart},
};
use askama::Template;
use crate::models::{Invitation, Person, EventDetails, Quote, GiftAccount, RsvpForm, InvitationRow, Song, User};
use crate::AppState;
use serde_json::{from_value, json};
use sqlx::Row;
use oauth2::{AuthorizationCode, TokenResponse, CsrfToken, Scope};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar};
use serde::Deserialize;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "invitation/create.html")]
pub struct CreateInvitationTemplate {
    pub user: Option<User>,
}

pub async fn create_invitation_page(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    // Basic Auth Check
    let user = match jar.get("user_id") {
        Some(cookie) => {
            let uid = Uuid::parse_str(cookie.value()).ok();
            if let Some(id) = uid {
                sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None)
            } else {
                None
            }
        }
        None => return Redirect::to("/auth/google").into_response(),
    };

    if user.is_none() {
        return Redirect::to("/auth/google").into_response();
    }

    HtmlTemplate(CreateInvitationTemplate { user }).into_response()
}

pub async fn create_invitation(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user_id_str = match jar.get("user_id") {
        Some(c) => c.value().to_owned(),
        None => return Redirect::to("/auth/google").into_response(),
    };
    let user_id = Uuid::parse_str(&user_id_str).unwrap();

    let mut fields = HashMap::new();
    let mut photo_paths = HashMap::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        
        if name.ends_with("_photo") {
            let filename = Uuid::new_v4().to_string() + ".jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap();
            if !data.is_empty() {
                std::fs::write(&path, data).unwrap();
                photo_paths.insert(name, format!("/{}", path));
            }
        } else {
            let value = field.text().await.unwrap();
            fields.insert(name, value);
        }
    }

    // Insert into DB
    let slug = fields.get("slug").unwrap().to_string();
    let bride_data = json!(Person {
        name: fields.get("bride_name").cloned().unwrap_or_default(),
        full_name: fields.get("bride_full_name").cloned().unwrap_or_default(),
        father_name: fields.get("bride_father").cloned().unwrap_or_default(),
        mother_name: fields.get("bride_mother").cloned().unwrap_or_default(),
        image_url: photo_paths.get("bride_photo").cloned().unwrap_or_else(|| "/static/img/bride.jpg".to_string()),
    });
    
    let groom_data = json!(Person {
        name: fields.get("groom_name").cloned().unwrap_or_default(),
        full_name: fields.get("groom_full_name").cloned().unwrap_or_default(),
        father_name: fields.get("groom_father").cloned().unwrap_or_default(),
        mother_name: fields.get("groom_mother").cloned().unwrap_or_default(),
        image_url: photo_paths.get("groom_photo").cloned().unwrap_or_else(|| "/static/img/groom.jpg".to_string()),
    });

    let ceremony_data = json!(EventDetails {
        date: fields.get("event_date").cloned().unwrap_or_default(),
        time: "09:00 - selesai".to_string(), // Simplified
        venue: fields.get("ceremony_venue").cloned().unwrap_or_default(),
        address: fields.get("ceremony_address").cloned().unwrap_or_default(),
        maps_url: "".to_string(),
    });

    let reception_data = json!(EventDetails {
        date: fields.get("event_date").cloned().unwrap_or_default(),
        time: "11:00 - selesai".to_string(),
        venue: fields.get("reception_venue").cloned().unwrap_or_default(),
        address: fields.get("reception_address").cloned().unwrap_or_default(),
        maps_url: "".to_string(),
    });

    let quote_data = json!({
        "text": "Sesungguhnya dalam penciptaan langit dan bumi, dan silih bergantinya malam dan siang terdapat tanda-tanda bagi orang-orang yang berakal.",
        "source": "Ali Imran: 190"
    });

    let template_name = fields.get("template_name").cloned().unwrap_or_else(|| "vintage".to_string());

    sqlx::query(
        "INSERT INTO invitations (user_id, slug, couple_name_short, event_date, template_name, bride_data, groom_data, ceremony_data, reception_data, quote_data) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
    )
    .bind(user_id)
    .bind(&slug)
    .bind(fields.get("couple_name_short").unwrap())
    .bind(fields.get("event_date").unwrap())
    .bind(template_name)
    .bind(bride_data)
    .bind(groom_data)
    .bind(ceremony_data)
    .bind(reception_data)
    .bind(quote_data)
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to(&format!("/invitation/{}", slug)).into_response()
}

#[derive(Debug, Deserialize)]
struct GoogleUser {
    id: String,
    email: String,
    name: String,
    picture: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AuthRequest {
    code: String,
    state: String,
}

pub async fn google_login(State(state): State<AppState>) -> impl IntoResponse {
    let (auth_url, _csrf_token) = state
        .oauth
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
        .url();

    Redirect::to(auth_url.as_str())
}

pub async fn google_callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(query): Query<AuthRequest>,
) -> impl IntoResponse {
    let token_result = state
        .oauth
        .exchange_code(AuthorizationCode::new(query.code))
        .request_async(&state.http_client)
        .await;

    let token = match token_result {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to exchange token: {}", e)).into_response(),
    };

    let user_info = state.http_client
        .get("https://www.googleapis.com/oauth2/v1/userinfo")
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .unwrap()
        .json::<GoogleUser>()
        .await
        .unwrap();

    // Store or Update user in DB
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (google_id, email, name, avatar_url)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (google_id) DO UPDATE SET name = $3, avatar_url = $4
         RETURNING *"
    )
    .bind(&user_info.id)
    .bind(&user_info.email)
    .bind(&user_info.name)
    .bind(&user_info.picture)
    .fetch_one(&state.db)
    .await
    .unwrap();

    // Store user_id in cookie (set to / to be available site-wide)
    let jar = jar.add(Cookie::build(("user_id", user.id.to_string()))
        .path("/")
        .http_only(true)
        .permanent());

    (jar, Redirect::to("/")).into_response()
}

pub async fn logout(jar: PrivateCookieJar) -> impl IntoResponse {
    let jar = jar.remove(Cookie::from("user_id"));
    (jar, Redirect::to("/")).into_response()
}

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
    pub user: Option<User>,
    pub invitations: Vec<InvitationRow>,
}

#[derive(Template)]
#[template(path = "invitation/vintage.html")]
pub struct VintageTemplate {
    pub invitation: Invitation,
}

#[derive(Template)]
#[template(path = "invitation/minimalist.html")]
pub struct MinimalistTemplate {
    pub invitation: Invitation,
}

#[derive(Template)]
#[template(path = "invitation/noir.html")]
pub struct NoirTemplate {
    pub invitation: Invitation,
}

pub async fn home(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let mut user = None;
    let mut invitations = Vec::new();

    if let Some(cookie) = jar.get("user_id") {
        if let Ok(uid) = Uuid::parse_str(cookie.value()) {
            user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(uid)
                .fetch_optional(&state.db)
                .await
                .unwrap_or(None);

            if user.is_some() {
                invitations = sqlx::query_as::<_, InvitationRow>(
                    "SELECT * FROM invitations WHERE user_id = $1 ORDER BY created_at DESC"
                )
                .bind(uid)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
            }
        }
    }

    HtmlTemplate(HomeTemplate { user, invitations }).into_response()
}

pub async fn invitation_detail(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 1. Fetch active song from DB (global fallback)
    let active_song = sqlx::query_as::<_, Song>(
        "SELECT * FROM songs WHERE is_active = true LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await
    .unwrap_or_default();

    let song_url = active_song.as_ref()
        .map(|s| s.file_path.clone())
        .unwrap_or_else(|| "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3".to_string());

    // 2. Fetch from DB
    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT * FROM invitations WHERE slug = $1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(row)) => {
            // Fetch associated data
            let gift_accounts = sqlx::query_as::<_, GiftAccount>(
                "SELECT bank_name, account_number, account_holder FROM gift_accounts WHERE invitation_id = $1"
            )
            .bind(row.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let photos = sqlx::query(
                "SELECT url FROM invitation_photos WHERE invitation_id = $1 ORDER BY \"order\" ASC"
            )
            .bind(row.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
            
            let gallery_images: Vec<String> = photos.into_iter()
                .map(|p| p.get::<String, _>("url"))
                .collect();

            let invitation = Invitation {
                slug: row.slug,
                couple_name_short: row.couple_name_short,
                bride: from_value(row.bride_data).unwrap(),
                groom: from_value(row.groom_data).unwrap(),
                event_date: row.event_date,
                ceremony: from_value(row.ceremony_data).unwrap(),
                reception: from_value(row.reception_data).unwrap(),
                quote: from_value(row.quote_data).unwrap(),
                gallery_images,
                gift_accounts,
                song_url,
            };

            match row.template_name.as_str() {
                "minimalist" => HtmlTemplate(MinimalistTemplate { invitation }).into_response(),
                "noir" => HtmlTemplate(NoirTemplate { invitation }).into_response(),
                _ => HtmlTemplate(VintageTemplate { invitation }).into_response(),
            }
        },
        _ => {
            // Fallback for samples
            if slug.ends_with("-sample") || slug == "sample" {
                let (couple_name, template_name) = match slug.as_str() {
                    "minimalist-sample" => ("Julia & Romeo", "minimalist"),
                    "noir-sample" => ("Sara & Benjamin", "noir"),
                    _ => ("Romeo & Julia", "vintage"),
                };

                let invitation = Invitation {
                    slug: slug.clone(),
                    couple_name_short: couple_name.to_string(),
                    bride: Person {
                        name: if template_name == "minimalist" { "Julia".to_string() } else { "Julia".to_string() },
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
                    song_url,
                };
                
                match template_name {
                    "minimalist" => HtmlTemplate(MinimalistTemplate { invitation }).into_response(),
                    "noir" => HtmlTemplate(NoirTemplate { invitation }).into_response(),
                    _ => HtmlTemplate(VintageTemplate { invitation }).into_response(),
                }
            } else {
                (StatusCode::NOT_FOUND, "Invitation not found").into_response()
            }
        }
    }
}

pub async fn rsvp(
    State(state): State<AppState>,
    Form(payload): Form<RsvpForm>
) -> impl IntoResponse {
    println!("RSVP received: {:?}", payload);
    
    // Save RSVP to DB if invitation exists
    let _ = sqlx::query(
        "INSERT INTO rsvps (invitation_id, name, attendance, guests, message) 
         SELECT id, $1, $2, $3, $4 FROM invitations LIMIT 1"
    )
    .bind(&payload.name)
    .bind(&payload.attendance)
    .bind(payload.guests as i32)
    .bind(&payload.message)
    .execute(&state.db)
    .await;
    
    let text_color = if payload.attendance == "Hadir" { "#2e7d32" } else { "#c62828" };
    let status_msg = if payload.attendance == "Hadir" { 
        format!("akan hadir dengan {} tamu", payload.guests) 
    } else { 
        "tidak dapat hadir".to_string() 
    };

    Html(format!(
        r#"<div id="rsvp-response" class="animate__animated animate__fadeIn paper-bg" style="padding: 2rem; border-radius: 20px; color: {}; text-align: center; box-shadow: var(--shadow-medium); border: 2px solid {}; position: relative; overflow: hidden;">
            <div style="position: absolute; top: -10px; right: -10px; width: 60px; height: 60px; background: var(--img-corner) no-repeat; background-size: contain; opacity: 0.3; transform: rotate(90deg);"></div>
            <p style="font-size: 1.3rem; margin-bottom: 0.5rem;"><strong>Terima kasih, {}!</strong></p>
            <p class="serif" style="font-size: 1.1rem;">Konfirmasi Anda (<strong>{}</strong>) telah kami terima.</p>
            {}
        </div>"#,
        text_color,
        if payload.attendance == "Hadir" { "var(--color-accent-sage)" } else { "var(--color-accent-rose)" },
        payload.name,
        status_msg,
        if !payload.message.is_empty() {
            format!(r#"<div style="margin-top: 1.5rem; padding-top: 1rem; border-top: 1px dashed rgba(0,0,0,0.1); font-style: italic; font-family: var(--font-serif);">"{}"</div>"#, payload.message)
        } else {
            "".to_string()
        }
    ))
}
pub async fn sitemap(State(state): State<AppState>) -> impl IntoResponse {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
    <url>
        <loc>https://castellant.id/</loc>
        <changefreq>daily</changefreq>
        <priority>1.0</priority>
    </url>"#);

    // Dynamic: fetch slugs from DB for the sitemap
    let slugs = sqlx::query("SELECT slug FROM invitations LIMIT 100")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    for row in slugs {
        let slug: String = row.get("slug");
        xml.push_str(&format!(r#"
    <url>
        <loc>https://castellant.id/invitation/{}</loc>
        <changefreq>weekly</changefreq>
        <priority>0.8</priority>
    </url>"#, slug));
    }

    xml.push_str("\n</urlset>");

    Response::builder()
        .header("Content-Type", "application/xml")
        .body(xml)
        .unwrap()
}
