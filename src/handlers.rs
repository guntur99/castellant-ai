use axum::{
    response::{Html, IntoResponse, Response, Redirect},
    http::StatusCode,
    Form,
    extract::{State, Path, Query, Multipart},
    Json,
};
use askama::Template;
use crate::models::{Invitation, Person, EventDetails, Quote, GiftAccount, RsvpForm, Rsvp, InvitationRow, Song, User, AiSession, Guest, GuestGroup, Booking, Voucher};
use crate::AppState;
use crate::mailer::{self, PaymentSuccessEmail};
use serde_json::{from_value, json};
use sqlx::Row;
use oauth2::{AuthorizationCode, TokenResponse, CsrfToken, Scope};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, CookieJar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Serialize)]
struct MayarInvoiceRequest {
    name: String,
    email: String,
    amount: i32,
    description: String,
    mobile: String,
    #[serde(rename = "redirectUrl")]
    redirect_url: String,
    items: Vec<MayarItem>,
    #[serde(rename = "extraData")]
    extra_data: HashMap<String, String>,
}

#[derive(Serialize)]
struct MayarItem {
    quantity: i32,
    rate: i32,
    description: String,
}

#[derive(Deserialize)]
struct MayarInvoiceResponse {
    #[allow(dead_code)]
    #[serde(rename = "statusCode", default)]
    status_code: i32,
    #[allow(dead_code)]
    #[serde(default)]
    status: bool,
    #[allow(dead_code)]
    #[serde(default)]
    messages: Option<String>,
    data: Option<serde_json::Value>,
}

#[derive(Template)]
#[template(path = "invitation/create.html")]
pub struct CreateInvitationTemplate {
    pub user: Option<User>,
    pub all_templates: Vec<TemplateMetadata>,
    pub is_dev: bool,
}

#[derive(Deserialize)]
pub struct PreviewRequest {
    pub template_name: String,
    pub couple_name_short: String,
    pub bride_name: String,
    pub bride_full_name: String,
    pub groom_name: String,
    pub groom_full_name: String,
    pub bride_father: String,
    pub bride_mother: String,
    pub groom_father: String,
    pub groom_mother: String,
    pub ceremony_date: String,
    pub ceremony_time: String,
    pub ceremony_venue: String,
    pub ceremony_address: String,
    pub ceremony_maps: String,
    pub reception_date: String,
    pub reception_time: String,
    pub reception_venue: String,
    pub reception_address: String,
    pub reception_maps: String,
    pub quote_text: String,
    pub quote_source: String,
}

#[derive(Deserialize)]
pub struct AiGenerateRequest {
    pub prompt: String,
    pub session_id: Option<Uuid>,
    pub context: Option<String>,
    pub guest_slug: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AiGenerateResponse {
    pub text: String,
    pub session_id: Option<Uuid>,
}

#[derive(Serialize, Clone, sqlx::FromRow, Debug)]
pub struct TemplateMetadata {
    pub id: String,
    pub slug: String,
    pub title: String,
    #[sqlx(rename = "description")]
    pub desc: String,
    pub category: String,
    pub preview_img: String,
    pub status: String,
    pub is_featured: bool,
}

#[derive(Template)]
#[template(path = "invitation/templates_list.html")]
pub struct TemplatesListTemplate {
    pub user: Option<User>,
    pub active_category: String,
    pub templates: Vec<TemplateMetadata>,
    pub current_page: i32,
    pub total_pages: i32,
    pub search_query: String,
    pub is_dev: bool,
}

pub async fn get_all_templates(db: &sqlx::PgPool, only_published: bool) -> Vec<TemplateMetadata> {
    let query = if only_published {
        "SELECT * FROM templates WHERE status = 'PUBLISHED' ORDER BY created_at DESC"
    } else {
        "SELECT * FROM templates ORDER BY created_at DESC"
    };
    sqlx::query_as::<_, TemplateMetadata>(query)
        .fetch_all(db)
        .await
        .unwrap_or_default()
}

pub async fn templates_list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let templates_data = get_all_templates(&state.db, true).await;
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
        None => None,
    };

    let category = params.get("category").cloned().unwrap_or_else(|| "all".to_string());
    let search = params.get("search").cloned().unwrap_or_default().to_lowercase();
    let page = params.get("page").and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
    let per_page = 6;

    let filtered: Vec<TemplateMetadata> = templates_data.into_iter()
        .filter(|t| category == "all" || t.category == category)
        .filter(|t| search.is_empty() || t.title.to_lowercase().contains(&search) || t.desc.to_lowercase().contains(&search))
        .collect();

    let total_pages = ((filtered.len() as f32) / (per_page as f32)).ceil() as i32;
    let start_idx = ((page - 1) * per_page) as usize;
    let paginated = filtered.into_iter()
        .skip(start_idx)
        .take(per_page as usize)
        .collect();

    HtmlTemplate(TemplatesListTemplate { 
        user, 
        active_category: category,
        templates: paginated,
        current_page: page,
        total_pages,
        search_query: search,
        is_dev: state.is_dev
    }).into_response()
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
        None => {
            if state.is_dev {
                return Redirect::to("/auth/mock").into_response();
            } else {
                return Redirect::to("/auth/google").into_response();
            }
        }
    };

    if user.is_none() {
        if state.is_dev {
            return Redirect::to("/auth/mock").into_response();
        } else {
            return Redirect::to("/auth/google").into_response();
        }
    }

    let all_templates = get_all_templates(&state.db, true).await;

    HtmlTemplate(CreateInvitationTemplate { 
        user, 
        all_templates,
        is_dev: state.is_dev 
    }).into_response()
}

pub async fn create_invitation(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user_id_str = match jar.get("user_id") {
        Some(c) => c.value().to_owned(),
        None => {
            if state.is_dev {
                return Redirect::to("/auth/mock").into_response();
            } else {
                return Redirect::to("/auth/google").into_response();
            }
        }
    };
    let user_id = match Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    let mut fields = HashMap::new();
    let mut photo_paths = HashMap::new();
    let mut gallery_paths = Vec::new();
    let mut bank_names = Vec::new();
    let mut account_numbers = Vec::new();
    let mut account_holders = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = match field.name() {
            Some(n) => n.to_string(),
            None => continue,
        };
        
        if name == "gallery[]" || name == "gallery_photo" {
            let filename = Uuid::new_v4().to_string() + ".jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                let _ = std::fs::create_dir_all("static/uploads");
                let _ = std::fs::write(&path, data);
                gallery_paths.push(format!("/{}", path));
            }
        } else if name.ends_with("_photo") {
            let filename = Uuid::new_v4().to_string() + ".jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                let _ = std::fs::create_dir_all("static/uploads");
                let _ = std::fs::write(&path, data);
                photo_paths.insert(name, format!("/{}", path));
            }
        } else if name == "payment_proof" {
            let filename = Uuid::new_v4().to_string() + "_payment.jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                let _ = std::fs::create_dir_all("static/uploads");
                let _ = std::fs::write(&path, data);
                fields.insert("payment_proof".to_string(), format!("/{}", path));
            }
        } else if name == "bank_name[]" {
            bank_names.push(field.text().await.unwrap_or_default());
        } else if name == "account_number[]" {
            account_numbers.push(field.text().await.unwrap_or_default());
        } else if name == "account_holder[]" {
            account_holders.push(field.text().await.unwrap_or_default());
        } else {
            let value = field.text().await.unwrap_or_default();
            fields.insert(name, value);
        }
    }

    // Insert into DB
    let slug = match fields.get("slug") {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, "Missing slug").into_response(),
    };

    // Ensure slug is unique
    let count: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&state.db)
        .await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Database error checking slug: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
            }
        };

    if count > 0 {
        return (StatusCode::BAD_REQUEST, "Slug already exists. Please choose another one.").into_response();
    }
    
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
        time: "09:00 - selesai".to_string(),
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
        "text": fields.get("quote_text").cloned().unwrap_or_else(|| "Sesungguhnya dalam penciptaan langit dan bumi...".to_string()),
        "source": fields.get("quote_source").cloned().unwrap_or_else(|| "Ali Imran: 190".to_string())
    });

    let plan_name = fields.get("plan_name").cloned().unwrap_or_else(|| "NOBLE".to_string());
    let amount = match plan_name.as_str() {
        "ROYAL" => 100000,
        "DYNASTY" => 300000,
        _ => 50000,
    };

    let user_row = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await {
            Ok(u) => u,
            Err(e) => {
                eprintln!("Failed to fetch user {}: {}", user_id, e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "User not found").into_response();
            }
        };

    let mut extra_data = HashMap::new();
    extra_data.insert("invitation_slug".to_string(), slug.clone());
    extra_data.insert("target_plan".to_string(), plan_name.clone());

    let items = vec![MayarItem {
        quantity: 1,
        rate: amount,
        description: format!("Digital Invitation - {} Plan", plan_name.to_uppercase()),
    }];

    // Call Mayar API
    let mayar_req = MayarInvoiceRequest {
        name: user_row.name.clone().unwrap_or_else(|| "Customer".to_string()),
        email: user_row.email.clone(),
        amount,
        description: format!("Digital Invitation - {} Plan ({})", plan_name.to_uppercase(), fields.get("couple_name_short").unwrap()),
        mobile: "08123456789".to_string(), // Fallback mobile
        redirect_url: format!("{}/invitation/{}/manage", std::env::var("REDIRECT_APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()), slug),
        items,
        extra_data,
    };

    let res = match state.http_client
        .post(&state.mayar_base_url)
        .header("Authorization", format!("Bearer {}", state.mayar_api_key))
        .json(&mayar_req)
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to send request to Mayar: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to connect to payment gateway").into_response();
            }
        };

    let status = res.status();
    let body_text = match res.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get body from Mayar response: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid response from payment gateway").into_response();
        }
    };
    
    let mayar_res: MayarInvoiceResponse = match serde_json::from_str(&body_text) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to decode Mayar response (status {}): {}. Body: {}", status, e, body_text);
            // If we can't decode it, let's see if it's a simple error message
            if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                if let Some(msg) = err_json.get("messages").and_then(|m| m.as_str()) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("Payment Gateway Error: {}", msg)).into_response();
                }
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to process payment response").into_response();
        }
    };

    let (payment_link, invoice_id) = if let Some(data) = mayar_res.data {
        let link = data.get("link").and_then(|l| l.as_str().map(|s| s.to_string()));
        let mut id = data.get("id").and_then(|i| i.as_str().map(|s| s.to_string()));
        
        // Extract readable ID from link (e.g., ix0x43nel8) to ensure better webhook matching
        if let Some(ref l) = link {
            if let Some(readable_id) = l.split('/').last() {
                // If we have a readable ID, let's use it as the primary identifier for bookings
                id = Some(readable_id.to_string());
            }
        }
        (link, id)
    } else {
        (None, None)
    };

    let template_name = fields.get("template_name").cloned().unwrap_or_else(|| "caiktok".to_string());
    let language = fields.get("language").cloned().unwrap_or_else(|| "id".to_string());
    let couple_name_short = fields.get("couple_name_short").cloned().unwrap_or_else(|| "Couple".to_string());
    let event_date = fields.get("event_date").cloned().unwrap_or_else(|| "TBA".to_string());

    // START TRANSACTION
    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let invitation_id = match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO invitations (user_id, slug, couple_name_short, event_date, template_name, bride_data, groom_data, ceremony_data, reception_data, quote_data, plan_name, language, payment_link, payment_invoice_id) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id"
    )
    .bind(user_id)
    .bind(&slug)
    .bind(&couple_name_short)
    .bind(&event_date)
    .bind(template_name)
    .bind(bride_data)
    .bind(groom_data)
    .bind(ceremony_data)
    .bind(reception_data)
    .bind(quote_data)
    .bind(plan_name.clone())
    .bind(language)
    .bind(payment_link.clone())
    .bind(invoice_id.clone())
    .fetch_one(&mut *tx)
    .await {
        Ok(id) => id,
        Err(e) => {
            let _ = tx.rollback().await;
            eprintln!("Failed to insert invitation: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save invitation. Possible duplicate slug.").into_response();
        }
    };

    // Insert Gallery Photos
    for (i, path) in gallery_paths.into_iter().enumerate() {
        if let Err(e) = sqlx::query(
            "INSERT INTO invitation_photos (invitation_id, url, photo_type, \"order\") VALUES ($1, $2, $3, $4)"
        )
        .bind(invitation_id)
        .bind(path)
        .bind("gallery")
        .bind(i as i32)
        .execute(&mut *tx)
        .await {
            let _ = tx.rollback().await;
            eprintln!("Failed to insert photo: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save photos").into_response();
        }
    }

    // Insert Gift Accounts
    for i in 0..bank_names.len() {
        if !bank_names[i].is_empty() && !account_numbers[i].is_empty() {
            if let Err(e) = sqlx::query(
                "INSERT INTO gift_accounts (invitation_id, bank_name, account_number, account_holder) VALUES ($1, $2, $3, $4)"
            )
            .bind(invitation_id)
            .bind(&bank_names[i])
            .bind(&account_numbers[i])
            .bind(&account_holders[i])
            .execute(&mut *tx)
            .await {
                let _ = tx.rollback().await;
                eprintln!("Failed to insert gift account: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save gift accounts").into_response();
            }
        }
    }

    // Insert Booking record
    if let Some(inv_id) = invoice_id {
        if let Err(e) = sqlx::query(
            "INSERT INTO bookings (user_id, invitation_id, target_plan, amount, invoice_id, payment_link, status) 
             VALUES ($1, $2, $3, $4, $5, $6, 'PENDING')"
        )
        .bind(user_id)
        .bind(invitation_id)
        .bind(&plan_name)
        .bind(amount)
        .bind(inv_id)
        .bind(payment_link.clone())
        .execute(&mut *tx)
        .await {
            let _ = tx.rollback().await;
            eprintln!("Failed to insert booking: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create booking record").into_response();
        }
    }

    // COMMIT TRANSACTION
    if let Err(e) = tx.commit().await {
        eprintln!("Failed to commit transaction: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to finalize invitation").into_response();
    }

    if let Some(link) = payment_link {
        Redirect::to(&link).into_response()
    } else {
        Redirect::to(&format!("/invitation/{}", slug)).into_response()
    }
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

pub async fn mock_login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    // Create/Update a mock developer user
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (google_id, email, name, avatar_url)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (google_id) DO UPDATE SET name = $3, avatar_url = $4
         RETURNING *"
    )
    .bind("mock_id_123")
    .bind("dev@castellant.id")
    .bind("Architect")
    .bind("https://cdn-icons-png.flaticon.com/512/3135/3135715.png")
    .fetch_one(&state.db)
    .await
    .unwrap();

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
    #[allow(dead_code)]
    pub user: Option<User>,
    #[allow(dead_code)]
    pub invitations: Vec<InvitationRow>,
    pub templates: Vec<TemplateMetadata>,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/trendvibe.html")]
pub struct TrendVibeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/loveanthem.html")]
pub struct LoveAnthemTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/cinemarry.html")]
pub struct CineMarryTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-gojek.html")]
pub struct CaiRideTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/pinterlove.html")]
pub struct PinterLoveTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/shopee-live-wedding.html")]
pub struct ShopeeLiveWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/tiktok-live-wedding.html")]
pub struct TiktokLiveWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-uber.html")]
pub struct WeUberTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-disney.html")]
pub struct WeddingDisneyTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-facebook.html")]
pub struct WeddingFacebookTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-iphone-theme.html")]
pub struct WeddingIphoneThemeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-netflix-v2.html")]
pub struct WeddingNetflixV2Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-prime.html")]
pub struct WeddingPrimeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-wrath-v2.html")]
pub struct WeddingWrathV2Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-applemusic.html")]
pub struct AppleMusicTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-capcut.html")]
pub struct WeCapCutTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/bereal-wedding.html")]
pub struct BeRealWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/instagram-live-wedding.html")]
pub struct InstagramLiveWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/qris-wedding.html")]
pub struct QrisWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-grab.html")]
pub struct WeddingGrabTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/figma-wedding.html")]
pub struct FigmaWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-vscode.html")]
pub struct WeVSCodeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-discord.html")]
pub struct WeDiscordTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-webtoon.html")]
pub struct WeWebtoonTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-manga.html")]
pub struct WeMangaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-nintendo-switch.html")]
pub struct WeNintendoSwitchTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-kai-v2.html")]
pub struct WeddingKaiV2Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-minecraft.html")]
pub struct WeddingMinecraftTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-zoom-v2.html")]
pub struct WeddingZoomV2Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-whatsapp-theme.html")]
pub struct WeddingWhatsappThemeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-mixue.html")]
pub struct WeMixueTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-playstation.html")]
pub struct WePlayStationTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/gmail-wedding.html")]
pub struct GmailWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-behance.html")]
pub struct WeBehanceTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-chatime.html")]
pub struct WeChatimeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-dribbble.html")]
pub struct WeDribbbleTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-hm.html")]
pub struct WeHMTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-janjijiwa.html")]
pub struct WeJanjiJiwaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-kopikenangan.html")]
pub struct WeKopiKenanganTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-powerpoint.html")]
pub struct WePowerPointTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-talenta.html")]
pub struct WeTalentaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-animal-crossing.html")]
pub struct WeddingAnimalCrossingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-claude.html")]
pub struct WeddingClaudeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-cod.html")]
pub struct WeddingCodTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-danamon.html")]
pub struct WeddingDanamonTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-excel-theme.html")]
pub struct WeddingExcelThemeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-freefire.html")]
pub struct WeddingFreeFireTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-github.html")]
pub struct WeddingGithubTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-jenius-v2.html")]
pub struct WeddingJeniusV2Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-linux.html")]
pub struct WeddingLinuxTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-word-theme.html")]
pub struct WeddingWordThemeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/canva-elegant-wedding.html")]
pub struct CanvaElegantWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/elegant-wedding.html")]
pub struct ElegantWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/mrt-wedding.html")]
pub struct MrtWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-brimo.html")]
pub struct WeBrimoTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-duolingo.html")]
pub struct WeDuolingoTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-google-calendar.html")]
pub struct WeGoogleCalendarTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-livin.html")]
pub struct WeLivinTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-manhua.html")]
pub struct WeManhuaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-manhwa.html")]
pub struct WeManhwaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-momoyo.html")]
pub struct WeMomoyoTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-steam-store.html")]
pub struct WeSteamStoreTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-uniqlo.html")]
pub struct WeUniqloTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-zara.html")]
pub struct WeZaraTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-bpjs.html")]
pub struct WeddingBpjsTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-chatgpt.html")]
pub struct WeddingChatGptTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-familymart.html")]
pub struct WeddingFamilyMartTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-gemini.html")]
pub struct WeddingGeminiTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-genshin-theme.html")]
pub struct WeddingGenshinThemeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-indomaret.html")]
pub struct WeddingIndomaretTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-jago.html")]
pub struct WeddingJagoTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-macintosh.html")]
pub struct WeddingMacintoshTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-mlbb.html")]
pub struct WeddingMlbbTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-ps5.html")]
pub struct WeddingPs5Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-pubg.html")]
pub struct WeddingPubgTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-telegram-theme.html")]
pub struct WeddingTelegramThemeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-wa-channel.html")]
pub struct WeddingWaChannelTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-windows95.html")]
pub struct WeddingWindows95Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-windowsxp.html")]
pub struct WeddingWindowsXpTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/whoosh-wedding.html")]
pub struct WhooshWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-threads-app.html")]
pub struct WeThreadsAppTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-alfamart.html")]
pub struct WeddingAlfamartTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-kai.html")]
pub struct WeddingKaiTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-medium.html")]
pub struct WeddingMediumTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-transjakarta.html")]
pub struct WeddingTransJakartaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/absensi-wedding.html")]
pub struct AbsensiWeddingTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-asana.html")]
pub struct WeAsanaTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-kopijago.html")]
pub struct WeKopiJagoTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-linktree.html")]
pub struct WeLinktreeTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/we-upwork.html")]
pub struct WeUpworkTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-danantara.html")]
pub struct WeddingDanantaraTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-dota2.html")]
pub struct WeddingDota2Template {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/wedding-indomie-goreng.html")]
pub struct WeddingIndomieGorengTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/manage.html")]
pub struct ManageInvitationTemplate {
    pub invitation: Invitation,
    pub all_templates: Vec<TemplateMetadata>,
    pub is_dev: bool,
    pub user: Option<User>,
    pub guests: Vec<Guest>,
    pub groups: Vec<GuestGroup>,
    pub rsvps: Vec<Rsvp>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub invitations: Vec<Invitation>,
    pub user: Option<User>,
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "profile.html")]
pub struct ProfileTemplate {
    pub user: Option<User>,
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub user: Option<User>,
    pub is_dev: bool,
}


pub async fn dashboard(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else {
        None
    };

    if let Some(uid) = user_id {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

        let invitations = sqlx::query_as::<_, InvitationRow>("SELECT * FROM invitations WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(uid)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| Invitation {
                slug: r.slug,
                template_name: r.template_name,
                couple_name_short: r.couple_name_short,
                bride: from_value(r.bride_data).unwrap_or_default(),
                groom: from_value(r.groom_data).unwrap_or_default(),
                event_date: r.event_date,
                ceremony: from_value(r.ceremony_data).unwrap_or_default(),
                reception: from_value(r.reception_data).unwrap_or_default(),
                quote: from_value(r.quote_data).unwrap_or_default(),
                gallery_images: Vec::new(),
                gift_accounts: Vec::new(),
                song_url: String::new(),
                plan_name: r.plan_name.unwrap_or_else(|| "NOBLE".to_string()),
                ai_chat_enabled: r.ai_chat_enabled,
                ai_usage_count: r.ai_usage_count,
                ai_custom_knowledge: r.ai_custom_knowledge.unwrap_or_default(),
                ai_language: r.ai_language.clone(),
                recipient_name: "Guest Guest & Partner".to_string(),
                event_date_iso: "2026-05-24T08:00:00".to_string(),
                rsvps: Vec::new(),
                is_preview: false,
            })
            .collect();

        HtmlTemplate(DashboardTemplate {
            invitations,
            user,
            is_dev: state.is_dev,
        }).into_response()
    } else {
        Redirect::to("/auth/google").into_response()
    }
}

pub async fn profile(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else {
        None
    };

    if let Some(uid) = user_id {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

        HtmlTemplate(ProfileTemplate {
            user,
            is_dev: state.is_dev,
        }).into_response()
    } else {
        Redirect::to("/auth/google").into_response()
    }
}

pub async fn settings(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else {
        None
    };

    if let Some(uid) = user_id {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

        HtmlTemplate(SettingsTemplate {
            user,
            is_dev: state.is_dev,
        }).into_response()
    } else {
        Redirect::to("/auth/google").into_response()
    }
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

    let templates = sqlx::query_as::<_, TemplateMetadata>(
        "SELECT * FROM templates WHERE status = 'PUBLISHED' AND is_featured = TRUE ORDER BY id ASC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    HtmlTemplate(HomeTemplate { user, invitations, templates, is_dev: state.is_dev }).into_response()
}

pub async fn invitation_detail(
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
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

            let mut template_name = row.template_name.clone();
            let mut ai_language = row.ai_language.clone();
            
            let mut recipient_name = "Guest Guest & Partner".to_string();
            
            // Override with preview_theme if provided
            if let Some(preview) = params.get("preview_theme") {
                template_name = preview.clone();
            } else if let Some(gs) = params.get("to") {
                recipient_name = gs.clone(); // Default to the query param value
                let guest = sqlx::query_as::<_, Guest>("SELECT * FROM guests WHERE invitation_id = $1 AND (slug = $2 OR name = $2)")
                    .bind(row.id)
                    .bind(gs)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or_default();
                
                if let Some(g) = guest {
                    recipient_name = g.name.clone();
                    // AI Language Override from Guest
                    if !g.ai_language.is_empty() {
                        ai_language = g.ai_language.clone();
                    }

                    let mut found_override = false;
                    
                    // 1. Check individual override
                    if let Some(t_override) = g.template_override {
                        if !t_override.is_empty() {
                            template_name = t_override;
                            found_override = true;
                        }
                    }
                    
                    // 2. Check group template if no individual override
                    if let Some(cat) = g.category {
                        let group = sqlx::query_as::<_, GuestGroup>("SELECT id, invitation_id, name, template_name, COALESCE(ai_language, '') as ai_language, created_at FROM invitation_groups WHERE invitation_id = $1 AND name = $2")
                            .bind(row.id)
                            .bind(&cat)
                            .fetch_optional(&state.db)
                            .await
                            .unwrap_or_default();
                        
                        if let Some(grp) = group {
                            if !found_override {
                                template_name = grp.template_name;
                            }
                            // If guest lang is empty, use group lang
                            if g.ai_language.is_empty() && !grp.ai_language.is_empty() {
                                ai_language = grp.ai_language;
                            }
                        }
                    }
                }
            }

            let event_date_iso = parse_event_date_to_iso(&row.event_date);

            let mut ceremony: EventDetails = from_value(row.ceremony_data).unwrap_or_default();
            let mut reception: EventDetails = from_value(row.reception_data).unwrap_or_default();
            
            // Format dates for display
            let event_date = format_date_for_display(&row.event_date);
            if ceremony.date.is_empty() || ceremony.date.contains('-') {
                ceremony.date = event_date.clone();
            } else {
                ceremony.date = format_date_for_display(&ceremony.date);
            }
            reception.date = format_date_for_display(&reception.date);

            let invitation = Invitation {
                slug: row.slug.clone(),
                template_name: template_name.clone(),
                couple_name_short: row.couple_name_short,
                bride: from_value(row.bride_data).unwrap_or_default(),
                groom: from_value(row.groom_data).unwrap_or_default(),
                event_date,
                ceremony,
                reception,
                quote: from_value(row.quote_data).unwrap_or_default(),
                gallery_images,
                gift_accounts,
                song_url,
                plan_name: row.plan_name.unwrap_or_else(|| "NOBLE".to_string()),
                ai_chat_enabled: row.ai_chat_enabled,
                ai_usage_count: row.ai_usage_count,
                ai_custom_knowledge: row.ai_custom_knowledge.unwrap_or_default(),
                ai_language: ai_language,
                recipient_name: recipient_name,
                event_date_iso: event_date_iso,
                rsvps: sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE invitation_id = $1 ORDER BY created_at DESC")
                    .bind(row.id)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default(),
                is_preview: params.contains_key("preview_theme"),
            };

            match template_name.as_str() {
                "loveanthem" => HtmlTemplate(LoveAnthemTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "cinemarry" => HtmlTemplate(CineMarryTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "cairide" => HtmlTemplate(CaiRideTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "pinterlove" => HtmlTemplate(PinterLoveTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "shopee-live-wedding" => HtmlTemplate(ShopeeLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "tiktok-live-wedding" => HtmlTemplate(TiktokLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-uber" => HtmlTemplate(WeUberTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-disney" => HtmlTemplate(WeddingDisneyTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-facebook" => HtmlTemplate(WeddingFacebookTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-iphone-theme" => HtmlTemplate(WeddingIphoneThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-netflix-v2" => HtmlTemplate(WeddingNetflixV2Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-prime" => HtmlTemplate(WeddingPrimeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-wrath-v2" => HtmlTemplate(WeddingWrathV2Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-applemusic" => HtmlTemplate(AppleMusicTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-capcut" => HtmlTemplate(WeCapCutTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "bereal-wedding" => HtmlTemplate(BeRealWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "instagram-live-wedding" => HtmlTemplate(InstagramLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-discord" => HtmlTemplate(WeDiscordTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-webtoon" => HtmlTemplate(WeWebtoonTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-mixue" => HtmlTemplate(WeMixueTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-playstation" => HtmlTemplate(WePlayStationTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-threads-app" => HtmlTemplate(WeThreadsAppTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-alfamart" => HtmlTemplate(WeddingAlfamartTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-kai" => HtmlTemplate(WeddingKaiTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-medium" => HtmlTemplate(WeddingMediumTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-transjakarta" => HtmlTemplate(WeddingTransJakartaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "qris-wedding" => HtmlTemplate(QrisWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-grab" => HtmlTemplate(WeddingGrabTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "figma-wedding" => HtmlTemplate(FigmaWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-whatsapp-theme" => HtmlTemplate(WeddingWhatsappThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-manga" => HtmlTemplate(WeMangaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-nintendo-switch" => HtmlTemplate(WeNintendoSwitchTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-kai-v2" => HtmlTemplate(WeddingKaiV2Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-minecraft" => HtmlTemplate(WeddingMinecraftTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-zoom-v2" => HtmlTemplate(WeddingZoomV2Template { invitation, is_dev: state.is_dev }).into_response(),
                "we-vscode" => HtmlTemplate(WeVSCodeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "gmail-wedding" => HtmlTemplate(GmailWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-behance" => HtmlTemplate(WeBehanceTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-chatime" => HtmlTemplate(WeChatimeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-dribbble" => HtmlTemplate(WeDribbbleTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-hm" => HtmlTemplate(WeHMTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-janjijiwa" => HtmlTemplate(WeJanjiJiwaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-kopikenangan" => HtmlTemplate(WeKopiKenanganTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-powerpoint" => HtmlTemplate(WePowerPointTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-talenta" => HtmlTemplate(WeTalentaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-animal-crossing" => HtmlTemplate(WeddingAnimalCrossingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-claude" => HtmlTemplate(WeddingClaudeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-cod" => HtmlTemplate(WeddingCodTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-danamon" => HtmlTemplate(WeddingDanamonTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-excel-theme" => HtmlTemplate(WeddingExcelThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-freefire" => HtmlTemplate(WeddingFreeFireTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-github" => HtmlTemplate(WeddingGithubTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-jenius-v2" => HtmlTemplate(WeddingJeniusV2Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-linux" => HtmlTemplate(WeddingLinuxTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-word-theme" => HtmlTemplate(WeddingWordThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "canva-elegant-wedding" => HtmlTemplate(CanvaElegantWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "elegant-wedding" => HtmlTemplate(ElegantWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "mrt-wedding" => HtmlTemplate(MrtWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-brimo" => HtmlTemplate(WeBrimoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-duolingo" => HtmlTemplate(WeDuolingoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-google-calendar" => HtmlTemplate(WeGoogleCalendarTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-livin" => HtmlTemplate(WeLivinTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-manhua" => HtmlTemplate(WeManhuaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-manhwa" => HtmlTemplate(WeManhwaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-momoyo" => HtmlTemplate(WeMomoyoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-steam-store" => HtmlTemplate(WeSteamStoreTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-uniqlo" => HtmlTemplate(WeUniqloTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-zara" => HtmlTemplate(WeZaraTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-bpjs" => HtmlTemplate(WeddingBpjsTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-chatgpt" => HtmlTemplate(WeddingChatGptTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-familymart" => HtmlTemplate(WeddingFamilyMartTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-gemini" => HtmlTemplate(WeddingGeminiTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-genshin-theme" => HtmlTemplate(WeddingGenshinThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-indomaret" => HtmlTemplate(WeddingIndomaretTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-jago" => HtmlTemplate(WeddingJagoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-macintosh" => HtmlTemplate(WeddingMacintoshTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-mlbb" => HtmlTemplate(WeddingMlbbTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-ps5" => HtmlTemplate(WeddingPs5Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-pubg" => HtmlTemplate(WeddingPubgTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-telegram-theme" => HtmlTemplate(WeddingTelegramThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-wa-channel" => HtmlTemplate(WeddingWaChannelTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-windows95" => HtmlTemplate(WeddingWindows95Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-windowsxp" => HtmlTemplate(WeddingWindowsXpTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "whoosh-wedding" => HtmlTemplate(WhooshWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "absensi-wedding" => HtmlTemplate(AbsensiWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-asana" => HtmlTemplate(WeAsanaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-kopijago" => HtmlTemplate(WeKopiJagoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-linktree" => HtmlTemplate(WeLinktreeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "we-upwork" => HtmlTemplate(WeUpworkTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-danantara" => HtmlTemplate(WeddingDanantaraTemplate { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-dota2" => HtmlTemplate(WeddingDota2Template { invitation, is_dev: state.is_dev }).into_response(),
                "wedding-indomie-goreng" => HtmlTemplate(WeddingIndomieGorengTemplate { invitation, is_dev: state.is_dev }).into_response(),
                _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
            }
        },
        _ => {
            // Fallback for samples
            if slug.ends_with("-sample") || slug == "sample" {
                let (couple_name, template_name) = if let Some(base) = slug.strip_suffix("-sample") {
                    ("Nazma & Guntur", base)
                } else {
                    match slug.as_str() {
                        "sample" => ("Nazma & Guntur", "trendvibe"),
                        _ => ("Nazma & Guntur", "trendvibe"),
                    }
                };

                let invitation = Invitation {
                    slug: slug.clone(),
                    template_name: template_name.to_string(),
                    couple_name_short: couple_name.to_string(),
                    bride: Person {
                        name: "Nazma".to_string(),
                        full_name: "Nazma Putri".to_string(),
                        father_name: "Bapak Nazma".to_string(),
                        mother_name: "Ibu Nazma".to_string(),
                        image_url: "/static/img/bride.jpg".to_string(),
                    },
                    groom: Person {
                        name: "Guntur".to_string(),
                        full_name: "Guntur Putra".to_string(),
                        father_name: "Bapak Guntur".to_string(),
                        mother_name: "Ibu Guntur".to_string(),
                        image_url: "/static/img/groom.jpg".to_string(),
                    },
                    event_date: "12 Desember 2026".to_string(),
                    ceremony: EventDetails {
                        date: "Sabtu, 12 Desember 2026".to_string(),
                        time: "09:00 - 10:00 WIB".to_string(),
                        venue: "Masjid Raya".to_string(),
                        address: "Jl. Diponegoro No.1, Jakarta".to_string(),
                        maps_url: "https://maps.app.goo.gl/xxx".to_string(),
                    },
                    reception: EventDetails {
                        date: "Sabtu, 12 Desember 2026".to_string(),
                        time: "11:00 - 13:00 WIB".to_string(),
                        venue: "Grand Ballroom".to_string(),
                        address: "Jl. Sudirman No.2, Jakarta".to_string(),
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
                            account_holder: "Nazma Putri".to_string(),
                        },
                    ],
                    song_url,
                    plan_name: "NOBLE".to_string(),
                    ai_chat_enabled: false,
                    ai_usage_count: 0,
                    ai_custom_knowledge: String::new(),
                    ai_language: "id".to_string(),
                    recipient_name: "Guest Guest & Partner".to_string(),
                    event_date_iso: "2026-12-12T08:00:00".to_string(),
                    rsvps: Vec::new(),
                    is_preview: true,
                };
                
                match template_name {
                    "loveanthem" => HtmlTemplate(LoveAnthemTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "cinemarry" => HtmlTemplate(CineMarryTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "cairide" => HtmlTemplate(CaiRideTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "pinterlove" => HtmlTemplate(PinterLoveTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "shopee-live-wedding" => HtmlTemplate(ShopeeLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "tiktok-live-wedding" => HtmlTemplate(TiktokLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-uber" => HtmlTemplate(WeUberTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-disney" => HtmlTemplate(WeddingDisneyTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-facebook" => HtmlTemplate(WeddingFacebookTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-iphone-theme" => HtmlTemplate(WeddingIphoneThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-netflix-v2" => HtmlTemplate(WeddingNetflixV2Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-prime" => HtmlTemplate(WeddingPrimeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-wrath-v2" => HtmlTemplate(WeddingWrathV2Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-applemusic" => HtmlTemplate(AppleMusicTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-capcut" => HtmlTemplate(WeCapCutTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "bereal-wedding" => HtmlTemplate(BeRealWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "instagram-live-wedding" => HtmlTemplate(InstagramLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "qris-wedding" => HtmlTemplate(QrisWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-grab" => HtmlTemplate(WeddingGrabTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "figma-wedding" => HtmlTemplate(FigmaWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-discord" => HtmlTemplate(WeDiscordTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-webtoon" => HtmlTemplate(WeWebtoonTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-mixue" => HtmlTemplate(WeMixueTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-playstation" => HtmlTemplate(WePlayStationTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-threads-app" => HtmlTemplate(WeThreadsAppTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-alfamart" => HtmlTemplate(WeddingAlfamartTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-kai" => HtmlTemplate(WeddingKaiTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-medium" => HtmlTemplate(WeddingMediumTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-transjakarta" => HtmlTemplate(WeddingTransJakartaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-whatsapp-theme" => HtmlTemplate(WeddingWhatsappThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-manga" => HtmlTemplate(WeMangaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-nintendo-switch" => HtmlTemplate(WeNintendoSwitchTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-kai-v2" => HtmlTemplate(WeddingKaiV2Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-minecraft" => HtmlTemplate(WeddingMinecraftTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-zoom-v2" => HtmlTemplate(WeddingZoomV2Template { invitation, is_dev: state.is_dev }).into_response(),
                    "we-vscode" => HtmlTemplate(WeVSCodeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "gmail-wedding" => HtmlTemplate(GmailWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-behance" => HtmlTemplate(WeBehanceTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-chatime" => HtmlTemplate(WeChatimeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-dribbble" => HtmlTemplate(WeDribbbleTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-hm" => HtmlTemplate(WeHMTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-janjijiwa" => HtmlTemplate(WeJanjiJiwaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-kopikenangan" => HtmlTemplate(WeKopiKenanganTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-powerpoint" => HtmlTemplate(WePowerPointTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-talenta" => HtmlTemplate(WeTalentaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-animal-crossing" => HtmlTemplate(WeddingAnimalCrossingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-claude" => HtmlTemplate(WeddingClaudeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-cod" => HtmlTemplate(WeddingCodTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-danamon" => HtmlTemplate(WeddingDanamonTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-excel-theme" => HtmlTemplate(WeddingExcelThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-freefire" => HtmlTemplate(WeddingFreeFireTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-github" => HtmlTemplate(WeddingGithubTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-jenius-v2" => HtmlTemplate(WeddingJeniusV2Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-linux" => HtmlTemplate(WeddingLinuxTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-word-theme" => HtmlTemplate(WeddingWordThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "canva-elegant-wedding" => HtmlTemplate(CanvaElegantWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "elegant-wedding" => HtmlTemplate(ElegantWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "mrt-wedding" => HtmlTemplate(MrtWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-brimo" => HtmlTemplate(WeBrimoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-duolingo" => HtmlTemplate(WeDuolingoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-google-calendar" => HtmlTemplate(WeGoogleCalendarTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-livin" => HtmlTemplate(WeLivinTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-manhua" => HtmlTemplate(WeManhuaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-manhwa" => HtmlTemplate(WeManhwaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-momoyo" => HtmlTemplate(WeMomoyoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-steam-store" => HtmlTemplate(WeSteamStoreTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-uniqlo" => HtmlTemplate(WeUniqloTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-zara" => HtmlTemplate(WeZaraTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-bpjs" => HtmlTemplate(WeddingBpjsTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-chatgpt" => HtmlTemplate(WeddingChatGptTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-familymart" => HtmlTemplate(WeddingFamilyMartTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-gemini" => HtmlTemplate(WeddingGeminiTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-genshin-theme" => HtmlTemplate(WeddingGenshinThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-indomaret" => HtmlTemplate(WeddingIndomaretTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-jago" => HtmlTemplate(WeddingJagoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-macintosh" => HtmlTemplate(WeddingMacintoshTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-mlbb" => HtmlTemplate(WeddingMlbbTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-ps5" => HtmlTemplate(WeddingPs5Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-pubg" => HtmlTemplate(WeddingPubgTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-telegram-theme" => HtmlTemplate(WeddingTelegramThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-wa-channel" => HtmlTemplate(WeddingWaChannelTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-windows95" => HtmlTemplate(WeddingWindows95Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-windowsxp" => HtmlTemplate(WeddingWindowsXpTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "whoosh-wedding" => HtmlTemplate(WhooshWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "absensi-wedding" => HtmlTemplate(AbsensiWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-asana" => HtmlTemplate(WeAsanaTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-kopijago" => HtmlTemplate(WeKopiJagoTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-linktree" => HtmlTemplate(WeLinktreeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "we-upwork" => HtmlTemplate(WeUpworkTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-danantara" => HtmlTemplate(WeddingDanantaraTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-dota2" => HtmlTemplate(WeddingDota2Template { invitation, is_dev: state.is_dev }).into_response(),
                    "wedding-indomie-goreng" => HtmlTemplate(WeddingIndomieGorengTemplate { invitation, is_dev: state.is_dev }).into_response(),
                    _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
                }
            } else {
                (StatusCode::NOT_FOUND, "Invitation not found").into_response()
            }
        }
    }
}

pub async fn manage_invitation(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let mut user_id = None;
    if let Some(cookie) = jar.get("user_id") {
        if let Ok(uid) = Uuid::parse_str(cookie.value()) {
            user_id = Some(uid);
        }
    }

    if user_id.is_none() {
        return Redirect::to("/").into_response();
    }

    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT * FROM invitations WHERE slug = $1 AND user_id = $2"
    )
    .bind(&slug)
    .bind(user_id.unwrap())
    .fetch_optional(&state.db)
    .await
    .unwrap();

    match row {
        Some(row) => {
            let event_date_iso = parse_event_date_to_iso(&row.event_date);
            let invitation = Invitation {
                slug: row.slug,
                template_name: row.template_name,
                couple_name_short: row.couple_name_short,
                bride: from_value(row.bride_data).unwrap_or_default(),
                groom: from_value(row.groom_data).unwrap_or_default(),
                event_date: row.event_date,
                ceremony: from_value(row.ceremony_data).unwrap_or_default(),
                reception: from_value(row.reception_data).unwrap_or_default(),
                quote: from_value(row.quote_data).unwrap_or_default(),
                gallery_images: Vec::new(),
                gift_accounts: sqlx::query_as::<_, GiftAccount>("SELECT bank_name, account_number, account_holder FROM gift_accounts WHERE invitation_id = $1").bind(row.id).fetch_all(&state.db).await.unwrap_or_default(),
                song_url: String::new(),
                plan_name: row.plan_name.unwrap_or_else(|| "NOBLE".to_string()),
                ai_chat_enabled: row.ai_chat_enabled,
                ai_usage_count: row.ai_usage_count,
                ai_custom_knowledge: row.ai_custom_knowledge.unwrap_or_default(),
                ai_language: row.ai_language.clone(),
                recipient_name: "Guest Guest & Partner".to_string(),
                event_date_iso,
                rsvps: sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE invitation_id = $1 ORDER BY created_at DESC")
                    .bind(row.id)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default(),
                is_preview: false,
            };

            let user = if let Some(cookie) = jar.get("user_id") {
                if let Ok(uid) = Uuid::parse_str(cookie.value()) {
                    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                        .bind(uid)
                        .fetch_optional(&state.db)
                        .await
                        .unwrap_or(None)
                } else { None }
            } else { None };

            let all_templates = get_all_templates(&state.db, false).await;
            let guests = sqlx::query_as::<_, Guest>("SELECT id, invitation_id, name, category, template_override, slug, is_sent, COALESCE(ai_language, '') as ai_language, created_at FROM guests WHERE invitation_id = $1 ORDER BY created_at DESC")
                .bind(row.id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
            
            let groups = sqlx::query_as::<_, GuestGroup>("SELECT id, invitation_id, name, template_name, COALESCE(ai_language, '') as ai_language, created_at FROM invitation_groups WHERE invitation_id = $1 ORDER BY name ASC")
                .bind(row.id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

            let rsvps = sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE invitation_id = $1 ORDER BY created_at DESC")
                .bind(row.id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

            HtmlTemplate(ManageInvitationTemplate { 
                invitation, 
                all_templates, 
                is_dev: state.is_dev,
                user,
                guests,
                groups,
                rsvps,
            }).into_response()
        },
        None => (StatusCode::NOT_FOUND, "Invitation not found or unauthorized").into_response(),
    }
}

pub async fn update_invitation(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut user_id = None;
    if let Some(cookie) = jar.get("user_id") {
        if let Ok(uid) = Uuid::parse_str(cookie.value()) {
            user_id = Some(uid);
        }
    }

    if user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Please login first").into_response();
    }

    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT * FROM invitations WHERE slug = $1 AND user_id = $2"
    )
    .bind(&slug)
    .bind(user_id.unwrap())
    .fetch_optional(&state.db)
    .await
    .unwrap();

    if row.is_none() {
        return (StatusCode::NOT_FOUND, "Invitation not found").into_response();
    }

    let row = row.unwrap();
    let mut fields = HashMap::new();
    let mut photo_paths = HashMap::new();
    let mut gallery_paths = Vec::new();
    let mut bank_names = Vec::new();
    let mut account_numbers = Vec::new();
    let mut account_holders = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        
        if name == "gallery[]" {
            let filename = Uuid::new_v4().to_string() + ".jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap();
            if !data.is_empty() {
                std::fs::create_dir_all("static/uploads").unwrap();
                std::fs::write(&path, data).unwrap();
                gallery_paths.push(format!("/{}", path));
            }
        } else if name.ends_with("_photo") {
            let filename = Uuid::new_v4().to_string() + ".jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap();
            if !data.is_empty() {
                std::fs::create_dir_all("static/uploads").unwrap();
                std::fs::write(&path, data).unwrap();
                photo_paths.insert(name, format!("/{}", path));
            }
        } else if name == "bank_name[]" {
            bank_names.push(field.text().await.unwrap_or_default());
        } else if name == "account_number[]" {
            account_numbers.push(field.text().await.unwrap_or_default());
        } else if name == "account_holder[]" {
            account_holders.push(field.text().await.unwrap_or_default());
        } else {
            let value = field.text().await.unwrap();
            fields.insert(name, value);
        }
    }

    // Update JSON Data
    let mut bride: Person = from_value(row.bride_data).unwrap_or_default();
    if let Some(val) = fields.get("bride_name") { bride.name = val.clone(); }
    if let Some(val) = fields.get("bride_full_name") { bride.full_name = val.clone(); }
    if let Some(val) = fields.get("bride_father") { bride.father_name = val.clone(); }
    if let Some(val) = fields.get("bride_mother") { bride.mother_name = val.clone(); }
    if let Some(val) = photo_paths.get("bride_photo") { bride.image_url = val.clone(); }

    let mut groom: Person = from_value(row.groom_data).unwrap_or_default();
    if let Some(val) = fields.get("groom_name") { groom.name = val.clone(); }
    if let Some(val) = fields.get("groom_full_name") { groom.full_name = val.clone(); }
    if let Some(val) = fields.get("groom_father") { groom.father_name = val.clone(); }
    if let Some(val) = fields.get("groom_mother") { groom.mother_name = val.clone(); }
    if let Some(val) = photo_paths.get("groom_photo") { groom.image_url = val.clone(); }

    let mut ceremony: EventDetails = from_value(row.ceremony_data).unwrap_or_default();
    if let Some(val) = fields.get("ceremony_time") { ceremony.time = val.clone(); }
    if let Some(val) = fields.get("ceremony_venue") { ceremony.venue = val.clone(); }
    if let Some(val) = fields.get("ceremony_address") { ceremony.address = val.clone(); }
    if let Some(val) = fields.get("ceremony_maps") { ceremony.maps_url = val.clone(); }

    let mut reception: EventDetails = from_value(row.reception_data).unwrap_or_default();
    if let Some(val) = fields.get("reception_date") { reception.date = val.clone(); }
    if let Some(val) = fields.get("reception_time") { reception.time = val.clone(); }
    if let Some(val) = fields.get("reception_venue") { reception.venue = val.clone(); }
    if let Some(val) = fields.get("reception_address") { reception.address = val.clone(); }
    if let Some(val) = fields.get("reception_maps") { reception.maps_url = val.clone(); }

    let mut quote: Quote = from_value(row.quote_data).unwrap_or_default();
    if let Some(val) = fields.get("quote_text") { quote.text = val.clone(); }
    if let Some(val) = fields.get("quote_source") { quote.source = val.clone(); }

    let couple_name_short = fields.get("couple_name_short").cloned().unwrap_or(row.couple_name_short);
    let event_date = fields.get("event_date").cloned().unwrap_or(row.event_date);
    let ai_chat_enabled = fields.get("ai_chat_enabled").map(|v| v == "on").unwrap_or(false);
    let ai_custom_knowledge = fields.get("ai_custom_knowledge").cloned().unwrap_or(row.ai_custom_knowledge.unwrap_or_default());
    let template_name = fields.get("template_name").cloned().unwrap_or(row.template_name);
    let final_ai_language = fields.get("ai_language").cloned().unwrap_or(row.ai_language.clone());

    sqlx::query(
        "UPDATE invitations SET couple_name_short = $1, event_date = $2, bride_data = $3, groom_data = $4, ceremony_data = $5, reception_data = $6, quote_data = $7, ai_chat_enabled = $8, ai_custom_knowledge = $9, ai_language = $10, template_name = $11 WHERE id = $12"
    )
    .bind(couple_name_short)
    .bind(event_date)
    .bind(json!(bride))
    .bind(json!(groom))
    .bind(json!(ceremony))
    .bind(json!(reception))
    .bind(json!(quote))
    .bind(ai_chat_enabled)
    .bind(ai_custom_knowledge)
    .bind(final_ai_language)
    .bind(template_name)
    .bind(row.id)
    .execute(&state.db)
    .await
    .unwrap();

    // Handle Gallery
    if !gallery_paths.is_empty() {
        for (i, path) in gallery_paths.into_iter().enumerate() {
            sqlx::query(
                "INSERT INTO invitation_photos (invitation_id, url, \"order\") VALUES ($1, $2, $3)"
            )
            .bind(row.id)
            .bind(path)
            .bind(i as i32)
            .execute(&state.db)
            .await
            .unwrap();
        }
    }

    // Update Gift Accounts: Delete existing and insert new ones
    let _ = sqlx::query("DELETE FROM gift_accounts WHERE invitation_id = $1")
        .bind(row.id)
        .execute(&state.db)
        .await;

    for i in 0..bank_names.len() {
        if !bank_names[i].is_empty() && !account_numbers[i].is_empty() {
            let _ = sqlx::query(
                "INSERT INTO gift_accounts (invitation_id, bank_name, account_number, account_holder) VALUES ($1, $2, $3, $4)"
            )
            .bind(row.id)
            .bind(&bank_names[i])
            .bind(&account_numbers[i])
            .bind(&account_holders[i])
            .execute(&state.db)
            .await;
        }
    }

    Redirect::to(&format!("/invitation/{}/manage", slug)).into_response()
}

pub async fn update_theme(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Form(fields): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut user_id = None;
    if let Some(cookie) = jar.get("user_id") {
        if let Ok(uid) = Uuid::parse_str(cookie.value()) {
            user_id = Some(uid);
        }
    }

    if user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Please login first").into_response();
    }

    let template_name = fields.get("template_name").cloned().unwrap_or_default();
    
    sqlx::query(
        "UPDATE invitations SET template_name = $1 WHERE slug = $2 AND user_id = $3"
    )
    .bind(template_name)
    .bind(&slug)
    .bind(user_id.unwrap())
    .execute(&state.db)
    .await
    .unwrap();

    // Trigger full page reload for HTMX
    [("HX-Refresh", "true")].into_response()
}

pub async fn rsvp(
    State(state): State<AppState>,
    Form(payload): Form<RsvpForm>
) -> impl IntoResponse {
    println!("RSVP received: {:?}", payload);
    
    // Save RSVP to DB if invitation exists
    let _ = sqlx::query(
        "INSERT INTO rsvps (invitation_id, name, attendance, guests, message) 
         SELECT id, $1, $2, $3, $4 FROM invitations WHERE slug = $5"
    )
    .bind(&payload.name)
    .bind(&payload.attendance)
    .bind(payload.guests as i32)
    .bind(&payload.message)
    .bind(&payload.invitation_slug)
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
pub async fn preview(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<PreviewRequest>,
) -> impl IntoResponse {
    let invitation = Invitation {
        slug: "preview".to_string(),
        template_name: payload.template_name.clone(),
        couple_name_short: payload.couple_name_short,
        bride: Person {
            name: payload.bride_name,
            full_name: payload.bride_full_name,
            father_name: payload.bride_father,
            mother_name: payload.bride_mother,
            image_url: "/static/img/bride.jpg".to_string(),
        },
        groom: Person {
            name: payload.groom_name,
            full_name: payload.groom_full_name,
            father_name: payload.groom_father,
            mother_name: payload.groom_mother,
            image_url: "/static/img/groom.jpg".to_string(),
        },
        event_date: payload.ceremony_date.clone(),
        ceremony: EventDetails {
            date: payload.ceremony_date,
            time: payload.ceremony_time,
            venue: payload.ceremony_venue,
            address: payload.ceremony_address,
            maps_url: payload.ceremony_maps,
        },
        reception: EventDetails {
            date: payload.reception_date,
            time: payload.reception_time,
            venue: payload.reception_venue,
            address: payload.reception_address,
            maps_url: payload.reception_maps,
        },
        quote: Quote {
            text: payload.quote_text,
            source: payload.quote_source,
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
                account_holder: "Preview User".to_string(),
            },
        ],
        song_url: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3".to_string(),
        plan_name: "NOBLE".to_string(),
        ai_chat_enabled: false,
        ai_usage_count: 0,
        ai_custom_knowledge: String::new(),
        ai_language: "id".to_string(),
        recipient_name: "Guest Guest & Partner".to_string(),
        event_date_iso: "2026-05-24T08:00:00".to_string(),
        rsvps: Vec::new(),
        is_preview: true,
    };

    match payload.template_name.as_str() {
        "loveanthem" => HtmlTemplate(LoveAnthemTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "cinemarry" => HtmlTemplate(CineMarryTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "cairide" => HtmlTemplate(CaiRideTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "pinterlove" => HtmlTemplate(PinterLoveTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "shopee-live-wedding" => HtmlTemplate(ShopeeLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "tiktok-live-wedding" => HtmlTemplate(TiktokLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-uber" => HtmlTemplate(WeUberTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-disney" => HtmlTemplate(WeddingDisneyTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-facebook" => HtmlTemplate(WeddingFacebookTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-iphone-theme" => HtmlTemplate(WeddingIphoneThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-netflix-v2" => HtmlTemplate(WeddingNetflixV2Template { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-prime" => HtmlTemplate(WeddingPrimeTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-wrath-v2" => HtmlTemplate(WeddingWrathV2Template { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-applemusic" => HtmlTemplate(AppleMusicTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-capcut" => HtmlTemplate(WeCapCutTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "bereal-wedding" => HtmlTemplate(BeRealWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "instagram-live-wedding" => HtmlTemplate(InstagramLiveWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "qris-wedding" => HtmlTemplate(QrisWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-grab" => HtmlTemplate(WeddingGrabTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "figma-wedding" => HtmlTemplate(FigmaWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-discord" => HtmlTemplate(WeDiscordTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-webtoon" => HtmlTemplate(WeWebtoonTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-whatsapp-theme" => HtmlTemplate(WeddingWhatsappThemeTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-mixue" => HtmlTemplate(WeMixueTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-playstation" => HtmlTemplate(WePlayStationTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-threads-app" => HtmlTemplate(WeThreadsAppTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-alfamart" => HtmlTemplate(WeddingAlfamartTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-kai" => HtmlTemplate(WeddingKaiTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-medium" => HtmlTemplate(WeddingMediumTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-transjakarta" => HtmlTemplate(WeddingTransJakartaTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-manga" => HtmlTemplate(WeMangaTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-nintendo-switch" => HtmlTemplate(WeNintendoSwitchTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-kai-v2" => HtmlTemplate(WeddingKaiV2Template { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-minecraft" => HtmlTemplate(WeddingMinecraftTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-zoom-v2" => HtmlTemplate(WeddingZoomV2Template { invitation, is_dev: state.is_dev }).into_response(),
        "we-vscode" => HtmlTemplate(WeVSCodeTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "absensi-wedding" => HtmlTemplate(AbsensiWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-asana" => HtmlTemplate(WeAsanaTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-kopijago" => HtmlTemplate(WeKopiJagoTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-linktree" => HtmlTemplate(WeLinktreeTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "we-upwork" => HtmlTemplate(WeUpworkTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-danantara" => HtmlTemplate(WeddingDanantaraTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-dota2" => HtmlTemplate(WeddingDota2Template { invitation, is_dev: state.is_dev }).into_response(),
        "wedding-indomie-goreng" => HtmlTemplate(WeddingIndomieGorengTemplate { invitation, is_dev: state.is_dev }).into_response(),
        _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
    }
}
pub async fn ai_generate_text(
    State(state): State<AppState>,
    Json(payload): Json<AiGenerateRequest>,
) -> impl IntoResponse {
    let api_key = &state.sumopod_api_key;
    let base_url = &state.sumopod_base_url;
    let model = &state.sumopod_model;

    if api_key.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "AI API Key not configured").into_response();
    }

    let messages = json!([
        {
            "role": "system",
            "content": "You are a professional wedding invitation copywriter. Your task is to generate romantic, elegant, and creative text for digital invitations. Keep it concise and suitable for the Indonesian market unless requested otherwise. Use a warm and sophisticated tone."
        },
        {
            "role": "user",
            "content": payload.prompt
        }
    ]);

    let body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.7
    });

    let res = state.http_client
        .post(base_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await;

    match res {
        Ok(resp) => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("Failed to generate content")
                .to_string();
            
            Json(AiGenerateResponse { text, session_id: None }).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("AI request failed: {}", e)).into_response()
        }
    }
}

pub async fn ai_guest_chat(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<AiGenerateRequest>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, InvitationRow>(
        "SELECT * FROM invitations WHERE slug = $1"
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await
    .unwrap();

    if row.is_none() {
        return (StatusCode::NOT_FOUND, "Invitation not found").into_response();
    }
    let invitation = row.unwrap();

    // Check Plan and Enablement
    let plan = invitation.plan_name.clone().unwrap_or_else(|| "NOBLE".to_string());
    if plan == "NOBLE" {
        return (StatusCode::FORBIDDEN, "AI Chat is not available in NOBLE plan. Please upgrade to ROYAL or DYNASTY.").into_response();
    }

    if !invitation.ai_chat_enabled {
        return (StatusCode::FORBIDDEN, "AI Chat is currently disabled for this invitation.").into_response();
    }

    // Check Limits
    let limit = if plan == "ROYAL" { 400 } else { 2500 };
    if invitation.ai_usage_count >= limit {
        return (StatusCode::PAYMENT_REQUIRED, "AI Chat limit reached for this invitation.").into_response();
    }

    let api_key = &state.sumopod_api_key;
    let base_url = &state.sumopod_base_url;
    let model = &state.sumopod_model;

    if api_key.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "AI API Key not configured").into_response();
    }

    let invitation_context = payload.context.unwrap_or_else(|| "No wedding details provided.".to_string());

    // Multi-level language fallback: Guest -> Group -> Invitation
    let mut final_ai_language = invitation.ai_language.clone();
    if final_ai_language.is_empty() { final_ai_language = "id".to_string(); }

    if let Some(g_slug) = payload.guest_slug {
        // Try to find specific guest language
        let guest_row = sqlx::query_as::<_, Guest>(
            "SELECT id, invitation_id, name, category, template_override, slug, is_sent, COALESCE(ai_language, '') as ai_language, created_at FROM guests WHERE invitation_id = $1 AND slug = $2"
        )
        .bind(invitation.id)
        .bind(&g_slug)
        .fetch_optional(&state.db)
        .await
        .unwrap_or_default();

        if let Some(guest) = guest_row {
            if !guest.ai_language.is_empty() {
                final_ai_language = guest.ai_language;
            } else {
                // If guest lang is empty, try to find group language
                if let Some(g_cat) = guest.category {
                    let group_row = sqlx::query_as::<_, GuestGroup>(
                        "SELECT id, invitation_id, name, template_name, COALESCE(ai_language, '') as ai_language, created_at FROM invitation_groups WHERE invitation_id = $1 AND name = $2"
                    )
                    .bind(invitation.id)
                    .bind(&g_cat)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or_default();

                    if let Some(group) = group_row {
                        if !group.ai_language.is_empty() {
                            final_ai_language = group.ai_language;
                        }
                    }
                }
            }
        }
    }

    let lang_str = match final_ai_language.as_str() {
        "en" => "English",
        "jv" => "Javanese (Bahasa Jawa)",
        "su" => "Sundanese (Bahasa Sunda)",
        "id" => "Bahasa Indonesia",
        "ja" => "Japanese (日本語)",
        "zh" => "Chinese (Mandarin)",
        "ko" => "Korean (한국어)",
        "ar" => "Arabic (العربية)",
        custom if !custom.is_empty() && custom != "custom" => custom,
        _ => "Bahasa Indonesia",
    };

    let messages = json!([
        {
            "role": "system",
            "content": format!(
                "You are a helpful Wedding Concierge. Use the following wedding details to answer guest questions. 
                Be polite, warm, and helpful. If you don't know the answer, politely ask them to contact the couple directly.
                
                ADDITIONAL KNOWLEDGE FROM THE COUPLE:
                {}

                STRICT BOUNDARIES: 
                - You ONLY answer questions about this specific wedding. 
                - DO NOT answer general questions, political questions, or technical questions.
                - If the question is not about this wedding, politely redirect the guest.
                - **CRITICAL**: YOU MUST ALWAYS RESPOND IN {}. Respond politely, warmly, and formally yet friendly in {}.

                WEDDING DETAILS:
                {}", 
                invitation.ai_custom_knowledge.unwrap_or_default(),
                lang_str,
                lang_str,
                invitation_context
            )
        },
        {
            "role": "user",
            "content": payload.prompt
        }
    ]);

    let body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.5
    });

    let res = state.http_client
        .post(base_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await;

    match res {
        Ok(resp) => {
            let json: serde_json::Value = resp.json().await.unwrap_or_default();
            let text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("I'm sorry, I couldn't process your request.")
                .to_string();
            
            // Increment usage count on success
            sqlx::query("UPDATE invitations SET ai_usage_count = ai_usage_count + 1 WHERE id = $1")
                .bind(invitation.id)
                .execute(&state.db)
                .await
                .unwrap();

            Json(AiGenerateResponse { text, session_id: None }).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("AI request failed: {}", e)).into_response()
        }
    }
}

pub async fn ai_parse_form(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<AiGenerateRequest>,
) -> impl IntoResponse {
    let api_key = std::env::var("SUMOPOD_API_KEY").unwrap_or_default();
    let base_url = std::env::var("SUMOPOD_BASE_URL").unwrap_or_default();
    let model = std::env::var("SUMOPOD_MODEL").unwrap_or_default();

    let auth_user_id = jar.get("session").and_then(|c| Uuid::parse_str(c.value()).ok());

    // 1. Get or Create Session
    let mut session_id = payload.session_id.unwrap_or_else(Uuid::new_v4);
    let mut history: Vec<serde_json::Value> = vec![];
    let mut current_form = serde_json::json!({});

    // Try finding session by ID or by User ID
    let session_res: Result<Option<AiSession>, sqlx::Error> = if let Some(uid) = auth_user_id {
        // If logged in, look for session by user_id first
        sqlx::query_as::<_, AiSession>("SELECT * FROM ai_sessions WHERE user_id = $1 ORDER BY updated_at DESC LIMIT 1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
    } else {
        // If guest, look by session_id
        sqlx::query_as::<_, AiSession>("SELECT * FROM ai_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_optional(&state.db)
            .await
    };

    match session_res {
        Ok(Some(session)) => {
            session_id = session.id;
            history = session.chat_history.as_array().cloned().unwrap_or_default();
            current_form = session.form_state;
            
            // If user just logged in, link the session
            if auth_user_id.is_some() && session.user_id.is_none() {
                let _ = sqlx::query("UPDATE ai_sessions SET user_id = $1 WHERE id = $2")
                    .bind(auth_user_id)
                    .bind(session_id)
                    .execute(&state.db)
                    .await;
            }
        }
        _ => {
            // Create new session in DB
            let _ = sqlx::query("INSERT INTO ai_sessions (id, user_id) VALUES ($1, $2)")
                .bind(session_id)
                .bind(auth_user_id)
                .execute(&state.db)
                .await;
        }
    }

    // 2. Prepare Messages for AI (same logic)
    let mut messages = vec![
        serde_json::json!({
            "role": "system",
            "content": format!("You are a proactive wedding assistant. 
            1. Extract these fields: couple_name_short, bride_name, bride_full_name, bride_father, bride_mother, groom_name, groom_full_name, groom_father, groom_mother, ceremony_date, ceremony_time, ceremony_venue, ceremony_address, reception_date, reception_time, reception_venue, reception_address, quote_text, quote_source.
            2. Current Form State: {}. Use this to know what's already filled.
            3. STRICT BOUNDARIES: 
               - You are ONLY a Wedding Invitation Assistant for Castellant.
               - DO NOT answer questions about politics, general knowledge, math, coding, or anything unrelated to this wedding form or Castellant services.
               - If the user goes off-topic, politely redirect them back to completing their wedding invitation.
            4. Return a JSON object with:
               - 'data': The extracted fields (merged with current state).
               - 'missing': A list of ALL fields that are still empty.
               - 'reply': A conversational reply in Indonesian. 
                 - ONLY ask for 2-4 missing fields per turn.
                 - Prioritize critical fields (Names, Date, Venue).
                 - Use a friendly 'korek info' tone.
                 - ALWAYS remind them about media (Gallery/Video) when text fields are nearly complete.
            5. Use YYYY-MM-DD for dates.
            6. If the user asks for random, dummy, or placeholder data (e.g., 'isi random', 'data dummy'), fill ALL empty fields with realistic dummy wedding data.
            7. ONLY return the JSON object.", current_form.to_string())
        })
    ];

    for h in &history { messages.push(h.clone()); }

    let user_msg = serde_json::json!({ "role": "user", "content": payload.prompt });
    messages.push(user_msg.clone());
    history.push(user_msg);

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.2,
        "response_format": { "type": "json_object" }
    });

    let res: Result<reqwest::Response, reqwest::Error> = state.http_client
        .post(base_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await;

    match res {
        Ok(resp) => {
            let json: serde_json::Value = resp.json::<serde_json::Value>().await.unwrap_or_default();
            let mut ai_text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("{}")
                .to_string();
            
            // Sanitize: Remove markdown code blocks if present
            if ai_text.starts_with("```json") {
                ai_text = ai_text.trim_start_matches("```json").trim_end_matches("```").trim().to_string();
            } else if ai_text.starts_with("```") {
                ai_text = ai_text.trim_start_matches("```").trim_end_matches("```").trim().to_string();
            }

            let parsed_ai: serde_json::Value = serde_json::from_str(&ai_text).unwrap_or_else(|_| serde_json::json!({
                "reply": "Maaf, saya mengalami kendala teknis saat memproses data. Bisa diulang?",
                "data": {},
                "missing": []
            }));
            
            let ai_reply = parsed_ai["reply"].as_str().unwrap_or("Done!").to_string();
            
            history.push(serde_json::json!({ "role": "assistant", "content": ai_reply }));
            
            let _ = sqlx::query("UPDATE ai_sessions SET chat_history = $1, form_state = $2, updated_at = NOW() WHERE id = $3")
                .bind(serde_json::to_value(&history).unwrap_or_default())
                .bind(&parsed_ai["data"])
                .bind(session_id)
                .execute(&state.db)
                .await;

            Json(AiGenerateResponse { 
                text: ai_text,
                session_id: Some(session_id)
            }).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("AI request failed: {}", e)).into_response()
        }
    }
}

pub async fn get_ai_session(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let session: Result<Option<AiSession>, sqlx::Error> = sqlx::query_as::<_, AiSession>("SELECT * FROM ai_sessions WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await;

    match session {
        Ok(Some(s)) => axum::Json(s).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Session not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct AddGuestRequest {
    pub name: String,
    pub category: Option<String>,
    pub template_override: Option<String>,
    pub ai_language: Option<String>,
}

pub async fn add_guest(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Form(payload): Form<AddGuestRequest>,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return Redirect::to("/").into_response(); }

    let invitation = sqlx::query!("SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2", slug, user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let guest_slug = payload.name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .replace(" ", "-");
    
    let ai_language = if invitation.plan_name.as_deref().unwrap_or("NOBLE") == "DYNASTY" {
        payload.ai_language.unwrap_or_default()
    } else {
        "".to_string()
    };

    sqlx::query(
        "INSERT INTO guests (invitation_id, name, category, slug, template_override, ai_language) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(invitation.id)
    .bind(&payload.name)
    .bind(&payload.category)
    .bind(&guest_slug)
    .bind(&payload.template_override)
    .bind(&ai_language)
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to(&format!("/invitation/{}/manage#guests", slug)).into_response()
}

pub async fn update_guest(
    Path((slug, guest_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Form(payload): Form<AddGuestRequest>,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return Redirect::to("/").into_response(); }

    let invitation = sqlx::query!("SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2", slug, user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let ai_language = if invitation.plan_name.as_deref().unwrap_or("NOBLE") == "DYNASTY" {
        payload.ai_language.unwrap_or_default()
    } else {
        "".to_string()
    };

    sqlx::query(
        "UPDATE guests SET name = $1, category = $2, template_override = $3, ai_language = $4 WHERE id = $5 AND invitation_id = $6"
    )
    .bind(&payload.name)
    .bind(&payload.category)
    .bind(&payload.template_override)
    .bind(&ai_language)
    .bind(guest_id)
    .bind(invitation.id)
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to(&format!("/invitation/{}/manage#guests", slug)).into_response()
}


#[derive(Deserialize)]
pub struct UpdateGuestTemplateRequest {
    pub template_override: String,
}

pub async fn update_guest_template(
    Path((slug, guest_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateGuestTemplateRequest>,
) -> impl IntoResponse {
    sqlx::query(
        "UPDATE guests SET template_override = $1 WHERE id = $2"
    )
    .bind(&payload.template_override)
    .bind(guest_id)
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to(&format!("/invitation/{}/manage#guests", slug)).into_response()
}

pub async fn delete_guest(
    Path((slug, guest_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM guests WHERE id = $1").bind(guest_id).execute(&state.db).await.unwrap();
    Redirect::to(&format!("/invitation/{}/manage#guests", slug)).into_response()
}

pub async fn delete_rsvp(
    Path((slug, rsvp_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return Redirect::to("/").into_response(); }

    sqlx::query("DELETE FROM rsvps WHERE id = $1 AND invitation_id = (SELECT id FROM invitations WHERE slug = $2 AND user_id = $3)")
        .bind(rsvp_id)
        .bind(&slug)
        .bind(user_id.unwrap())
        .execute(&state.db)
        .await
        .unwrap();

    Redirect::to(&format!("/invitation/{}/manage#rsvps", slug)).into_response()
}

#[derive(Deserialize)]
pub struct AddGroupRequest {
    pub name: String,
    pub template_name: String,
    pub ai_language: Option<String>,
}

pub async fn add_group(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Form(payload): Form<AddGroupRequest>,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return Redirect::to("/").into_response(); }

    let (invitation_id, plan_name): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2"
    )
    .bind(&slug)
    .bind(user_id.unwrap())
    .fetch_one(&state.db)
    .await
    .unwrap();

    let plan_name = plan_name.unwrap_or_else(|| "NOBLE".to_string());

    // Check if group already exists (for updates)
    let existing = sqlx::query!("SELECT id FROM invitation_groups WHERE invitation_id = $1 AND name = $2", invitation_id, payload.name)
        .fetch_optional(&state.db)
        .await
        .unwrap();

    if existing.is_none() {
        // Only check limit for NEW groups
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invitation_groups WHERE invitation_id = $1")
            .bind(invitation_id)
            .fetch_one(&state.db)
            .await
            .unwrap();

        let limit = match plan_name.as_str() {
            "ROYAL" => 7,
            "DYNASTY" => 999,
            _ => 3, // NOBLE
        };

        if count >= limit as i64 {
            return (StatusCode::FORBIDDEN, "Plan limit reached. Please upgrade to add more groups.").into_response();
        }
    }

    let ai_language = if plan_name == "ROYAL" || plan_name == "DYNASTY" {
        payload.ai_language.unwrap_or_default()
    } else {
        "".to_string()
    };

    sqlx::query(
        "INSERT INTO invitation_groups (invitation_id, name, template_name, ai_language) VALUES ($1, $2, $3, $4)
         ON CONFLICT (invitation_id, name) DO UPDATE SET template_name = $3, ai_language = $4"
    )
    .bind(invitation_id)
    .bind(&payload.name)
    .bind(&payload.template_name)
    .bind(&ai_language)
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to(&format!("/invitation/{}/manage#groups", slug)).into_response()
}

pub async fn update_group(
    Path((slug, group_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Form(payload): Form<AddGroupRequest>,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return Redirect::to("/").into_response(); }

    // Verify ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM invitations WHERE slug = $1 AND user_id = $2)"
    )
    .bind(&slug)
    .bind(user_id.unwrap())
    .fetch_one(&state.db)
    .await
    .unwrap();

    if !exists { return Redirect::to("/").into_response(); }

    sqlx::query(
        "UPDATE invitation_groups SET name = $1, template_name = $2, ai_language = $3 WHERE id = $4"
    )
    .bind(&payload.name)
    .bind(&payload.template_name)
    .bind(&payload.ai_language)
    .bind(group_id)
    .execute(&state.db)
    .await
    .unwrap();

    Redirect::to(&format!("/invitation/{}/manage#groups", slug)).into_response()
}



pub async fn delete_group(
    Path((slug, group_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM invitation_groups WHERE id = $1").bind(group_id).execute(&state.db).await.unwrap();
    Redirect::to(&format!("/invitation/{}/manage#groups", slug)).into_response()
}

#[derive(Deserialize)]
pub struct CreateUpgradePaymentRequest {
    pub target_plan: String,
    pub voucher_code: Option<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct TemplateLeaderboardEntry {
    pub template_name: String,
    #[sqlx(default)]
    pub slug: String,
    #[sqlx(default)]
    pub friendly_title: String,
    #[sqlx(default)]
    pub preview_url: String,
    pub count: i64,
}

#[derive(Template)]
#[template(path = "admin/revenue.html")]
pub struct AdminRevenueTemplate {
    pub user: Option<User>,
    pub total_revenue: i64,
    pub successful_bookings: i64,
    pub average_order_value: f64,
    pub bookings: Vec<Booking>,
    pub leaderboard: Vec<TemplateLeaderboardEntry>,
    pub is_dev: bool,
}

pub async fn admin_revenue(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = match jar.get("user_id") {
        Some(cookie) => {
            let uid = Uuid::parse_str(cookie.value()).ok();
            if let Some(id) = uid {
                sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None)
            } else { None }
        }
        None => None,
    };

    // Admin check
    if let Some(u) = &user {
        if u.role != "SUPERADMIN" {
            return (StatusCode::FORBIDDEN, "Admin access required").into_response();
        }
    } else {
        return Redirect::to("/").into_response();
    }

    let bookings = sqlx::query_as::<_, Booking>("SELECT * FROM bookings ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let total_revenue: i64 = bookings.iter()
        .filter(|b| b.status == "SUCCESS")
        .map(|b| (b.amount - b.discount_amount) as i64)
        .sum();

    let successful_count = bookings.iter().filter(|b| b.status == "SUCCESS").count() as i64;
    let avg_order = if successful_count > 0 { total_revenue as f64 / successful_count as f64 } else { 0.0 };

    let mut leaderboard = sqlx::query_as::<_, TemplateLeaderboardEntry>("SELECT template_name, COALESCE(COUNT(*), 0) as count FROM invitations GROUP BY template_name ORDER BY count DESC LIMIT 5")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // Map technical names to friendly titles, previews, and slugs
    let all_templates = get_all_templates(&state.db, false).await;
    for entry in leaderboard.iter_mut() {
        if let Some(meta) = all_templates.iter().find(|t| t.id == entry.template_name) {
            entry.friendly_title = meta.title.clone();
            entry.preview_url = meta.preview_img.clone();
            entry.slug = meta.slug.clone();
        } else {
            entry.friendly_title = entry.template_name.clone();
            entry.preview_url = "/static/img/placeholder.png".to_string();
            entry.slug = entry.template_name.clone();
        }
    }

    HtmlTemplate(AdminRevenueTemplate {
        user,
        total_revenue,
        successful_bookings: successful_count,
        average_order_value: avg_order,
        bookings,
        leaderboard,
        is_dev: state.is_dev,
    }).into_response()
}

#[derive(Template)]
#[template(path = "receipt.html")]
pub struct ReceiptTemplate {
    pub booking: Booking,
    pub invitation: InvitationRow,
}

pub async fn receipt_detail(
    Path(invoice_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let booking = sqlx::query_as::<_, Booking>("SELECT * FROM bookings WHERE invoice_id = $1")
        .bind(&invoice_id)
        .fetch_optional(&state.db)
        .await
        .unwrap();

    if let Some(b) = booking {
        let invitation = sqlx::query_as::<_, InvitationRow>("SELECT * FROM invitations WHERE id = $1")
            .bind(b.invitation_id)
            .fetch_one(&state.db)
            .await
            .unwrap();

        HtmlTemplate(ReceiptTemplate {
            booking: b,
            invitation,
        }).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Receipt not found").into_response()
    }
}

pub async fn create_upgrade_payment(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(payload): Json<CreateUpgradePaymentRequest>,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return (StatusCode::UNAUTHORIZED, "Login required").into_response(); }

    let invitation: InvitationRow = sqlx::query_as("SELECT * FROM invitations WHERE slug = $1 AND user_id = $2")
        .bind(&slug)
        .bind(user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let current_plan_price = match invitation.plan_name.as_deref().unwrap_or("NOBLE") {
        "ROYAL" => 100000,
        "DYNASTY" => 300000,
        _ => 50000,
    };

    let target_plan_price = match payload.target_plan.as_str() {
        "ROYAL" => 100000,
        "DYNASTY" => 300000,
        _ => 50000,
    };

    let amount = if target_plan_price > current_plan_price {
        target_plan_price - current_plan_price
    } else {
        0
    };

    let mut discount_amount = 0;
    let mut applied_voucher = None;

    if let Some(code) = &payload.voucher_code {
        if !code.is_empty() {
            let voucher = sqlx::query_as::<_, Voucher>("SELECT * FROM vouchers WHERE code = $1 AND is_active = true AND (usage_limit IS NULL OR usage_count < usage_limit) AND (valid_until IS NULL OR valid_until > NOW())")
                .bind(code)
                .fetch_optional(&state.db)
                .await
                .unwrap_or(None);
            
            if let Some(v) = voucher {
                discount_amount = (amount * v.discount_percent) / 100;
                applied_voucher = Some(v.code);
            }
        }
    }

    let final_amount = amount - discount_amount;

    let user_row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let mut extra_data = HashMap::new();
    extra_data.insert("invitation_slug".to_string(), slug.clone());
    extra_data.insert("target_plan".to_string(), payload.target_plan.clone());
    if let Some(code) = &applied_voucher {
        extra_data.insert("voucher_code".to_string(), code.clone());
    }

    let items = vec![MayarItem {
        quantity: 1,
        rate: final_amount,
        description: format!("Upgrade to {} Plan", payload.target_plan),
    }];

    let mayar_req = MayarInvoiceRequest {
        name: user_row.name.unwrap_or_else(|| "Customer".to_string()),
        email: user_row.email,
        amount: final_amount,
        description: format!("Upgrade to {} Plan - {}", payload.target_plan, invitation.couple_name_short),
        mobile: "08123456789".to_string(),
        redirect_url: format!("{}/invitation/{}/manage", std::env::var("REDIRECT_APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()), slug),
        items,
        extra_data,
    };

    let res = match state.http_client
        .post(&state.mayar_base_url)
        .header("Authorization", format!("Bearer {}", state.mayar_api_key))
        .json(&mayar_req)
        .send()
        .await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to send request to Mayar (upgrade): {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to connect to payment gateway").into_response();
            }
        };

    let status = res.status();
    let body_text = match res.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to get body from Mayar response (upgrade): {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid response from payment gateway").into_response();
        }
    };

    let mayar_res: MayarInvoiceResponse = match serde_json::from_str(&body_text) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to decode Mayar response (upgrade, status {}): {}. Body: {}", status, e, body_text);
            if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                if let Some(msg) = err_json.get("messages").and_then(|m| m.as_str()) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("Payment Gateway Error: {}", msg)).into_response();
                }
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to process payment response").into_response();
        }
    };

    if let Some(data) = mayar_res.data {
        let link = data.get("link").and_then(|l| l.as_str().map(|s| s.to_string()));
        let id = data.get("id").and_then(|i| i.as_str().map(|s| s.to_string()));

        sqlx::query("UPDATE invitations SET payment_link = $1, payment_invoice_id = $2 WHERE id = $3")
            .bind(&link)
            .bind(&id)
            .bind(invitation.id)
            .execute(&state.db)
            .await
            .unwrap();

        // 2. Track in Bookings Table
        sqlx::query("INSERT INTO bookings (user_id, invitation_id, target_plan, amount, invoice_id, payment_link, status, voucher_code, discount_amount) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(user_id)
            .bind(invitation.id)
            .bind(&payload.target_plan)
            .bind(amount)
            .bind(&id)
            .bind(&link)
            .bind("PENDING")
            .bind(applied_voucher)
            .bind(discount_amount)
            .execute(&state.db)
            .await
            .unwrap();
        
        Json(json!({ "status": "success", "link": link })).into_response()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create invoice").into_response()
    }
}

pub async fn mayar_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 1. Get Event and Data
    let event_raw = payload.get("event").and_then(|e| e.as_str()).unwrap_or("");
    let event_name = event_raw.to_lowercase();
    let data = payload.get("data");
    
    let data_id = data.and_then(|d| 
        d.get("id")
        .or_else(|| d.get("invoiceId"))
        .or_else(|| d.get("invoice_id"))
        .or_else(|| d.get("invoice_no"))
        .or_else(|| d.get("no_invoice"))
        .and_then(|i| i.as_str())
    );

    // 2. Verify Webhook Token
    let token = headers.get("Authorization")
        .or_else(|| headers.get("X-Mayar-Token"))
        .or_else(|| headers.get("x-mayar-token"))
        .and_then(|h| h.to_str().ok());
    
    let token_in_payload = payload.get("token").and_then(|t| t.as_str());
    
    let expected_token = std::env::var("MAYAR_WEBHOOK_SECRET").unwrap_or_default();
    let is_authorized = if expected_token.is_empty() {
        true // Allow if not configured
    } else {
        let provided = token.or(token_in_payload).unwrap_or("").trim().replace("Bearer ", "");
        provided == expected_token.trim()
    };
    
    tracing::info!("Received Webhook [{}]: event={}, data_id={:?}, auth={}", 
        if is_authorized { "AUTH" } else { "GUEST" },
        event_raw, 
        data_id,
        is_authorized
    );

    let mut is_valid_fallback = false;
    if !is_authorized {
        // Fallback: Check if the payload contains a valid slug and data that matches our DB
        if let Some(d) = data {
            let extra_data_val = d.get("extraData").or_else(|| d.get("extra_data"));
            if let Some(ed) = extra_data_val {
                let slug = ed.get("invitation_slug").and_then(|s| s.as_str());
                if let Some(s) = slug {
                    // Verify if this slug exists and has a pending booking
                    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM invitations WHERE slug = $1)")
                        .bind(s)
                        .fetch_one(&state.db)
                        .await
                        .unwrap_or(false);
                    if exists {
                        tracing::warn!("Unauthorized Webhook matched existing slug '{}'. Proceeding with caution.", s);
                        is_valid_fallback = true;
                    }
                }
            }
        }
    }

    if !is_authorized && !is_valid_fallback && !event_name.contains("testing") {
        let header_keys: Vec<String> = headers.keys().map(|k| k.to_string()).collect();
        tracing::warn!("401 Unauthorized Webhook: provided_prefix={:?}, expected_prefix={:?}, headers={:?}, payload_event={}", 
            token.map(|t| if t.len() > 8 { &t[..8] } else { t }),
            if expected_token.len() > 8 { Some(&expected_token[..8]) } else { None },
            header_keys,
            event_raw
        );
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // 3. Handle Payment Success
    if event_name.contains("payment.received") || event_name.contains("testing") {
        if let Some(d) = data {
            let extra_data_val = d.get("extraData").or_else(|| d.get("extra_data"));
            
            if let Some(ed) = extra_data_val {
                let slug = ed.get("invitation_slug").and_then(|s| s.as_str().map(|v| v.to_string()));
                let plan = ed.get("target_plan").and_then(|p| p.as_str().map(|v| v.to_string()));

                if let (Some(slug), Some(plan)) = (slug, plan) {
                    // Update Invitation Plan and Status
                    let _ = sqlx::query("UPDATE invitations SET plan_name = $1, payment_status = 'SUCCESS' WHERE slug = $2")
                        .bind(&plan)
                        .bind(&slug)
                        .execute(&state.db)
                        .await;
                    
                    tracing::info!("Payment success for {}: Plan upgraded to {}", slug, plan);

                    // Also try to update booking status using slug as fallback if ID fails later
                    let _ = sqlx::query("UPDATE bookings SET status = 'SUCCESS', updated_at = NOW() WHERE invitation_id = (SELECT id FROM invitations WHERE slug = $1)")
                        .bind(&slug)
                        .execute(&state.db)
                        .await;

                    // Send Email Notification
                    let user_info = sqlx::query!(
                         "SELECT u.email, u.name as user_name, i.couple_name_short, i.language 
                          FROM invitations i 
                          JOIN users u ON i.user_id = u.id 
                          WHERE i.slug = $1",
                         &slug
                    )
                    .fetch_optional(&state.db)
                    .await
                    .ok()
                    .flatten();

                    if let Some(info) = user_info {
                        let amount = d.get("amount").and_then(|a| a.as_i64()).unwrap_or(0) as i32;
                        let email_template = PaymentSuccessEmail {
                            name: info.user_name.unwrap_or_else(|| "Customer".to_string()),
                            plan_name: plan.to_uppercase(),
                            slug: slug.clone(),
                            amount,
                            language: info.language.unwrap_or_else(|| "id".to_string()),
                            base_url: std::env::var("REDIRECT_APP_BASE_URL").unwrap_or_else(|_| "https://castellant-ai.up.railway.app".to_string()),
                        };

                        let to_email = info.email.clone();
                        let slug_clone = slug.clone();
                        tokio::spawn(async move {
                            let mut success = false;
                            let mut last_err = String::new();
                            
                            // Try multiple ports if the default one fails
                            let ports = [None, Some(587), Some(465), Some(2525)];
                            
                            for (i, &port) in ports.iter().enumerate() {
                                match mailer::send_payment_success_email(&to_email, email_template.clone(), port).await {
                                    Ok(_) => {
                                        success = true;
                                        tracing::info!("Successfully sent payment success email for {} on attempt {} (port: {:?})", slug_clone, i + 1, port);
                                        break;
                                    },
                                    Err(e) => {
                                        last_err = e.clone();
                                        tracing::warn!("Email attempt {} failed for {} (port: {:?}): {}. Retrying with next port...", i + 1, slug_clone, port, e);
                                        // Short delay before next attempt
                                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                    }
                                }
                            }
                            
                            if !success {
                                tracing::error!("Failed to send payment success email for {} after trying all ports: {}", slug_clone, last_err);
                            }
                        });
                    }
                }
            }
        }
    }

    // 4. Update Booking Status if ID is present
    if let Some(id) = data_id {
        let status = if event_name.contains("payment.received") || event_name.contains("testing") {
            "SUCCESS"
        } else if event_name.contains("payment.failed") {
            "FAILED"
        } else {
            "PENDING"
        };
        
        if status != "PENDING" {
            let id_from_link = data.and_then(|d| d.get("link").and_then(|l| l.as_str()).and_then(|s| s.split('/').last()));

            // Match by invoice_id OR try to find by payment_link if it contains the ID
            let result = sqlx::query("UPDATE bookings SET status = $1, updated_at = NOW() WHERE invoice_id = $2 OR invoice_id = $3 OR payment_link LIKE $4")
                .bind(status)
                .bind(id)
                .bind(id_from_link.unwrap_or(""))
                .bind(format!("%{}%", id))
                .execute(&state.db)
                .await;
            
            match result {
                Ok(res) => {
                    if res.rows_affected() == 0 {
                        tracing::warn!("Webhook received for invoice_id {} but no matching booking found to update via ID", id);
                        
                        // Failsafe: Create a late-booking record if it's missing
                        if status == "SUCCESS" {
                             if let Some(d) = data {
                                let extra_data_val = d.get("extraData").or_else(|| d.get("extra_data"));
                                if let Some(ed) = extra_data_val {
                                    let slug = ed.get("invitation_slug").and_then(|s| s.as_str());
                                    let plan = ed.get("target_plan").and_then(|p| p.as_str());
                                    
                                    if let (Some(slug), Some(plan)) = (slug, plan) {
                                        sqlx::query("INSERT INTO bookings (invitation_id, target_plan, amount, invoice_id, status, created_at, updated_at) 
                                                     SELECT id, $1, $2, $3, $4, NOW(), NOW() FROM invitations WHERE slug = $5")
                                            .bind(plan)
                                            .bind(d.get("amount").and_then(|a| a.as_i64()).unwrap_or(0) as i32)
                                            .bind(id)
                                            .bind(status)
                                            .bind(slug)
                                            .execute(&state.db)
                                            .await
                                            .ok();
                                    }
                                }
                             }
                        }
                    } else {
                        tracing::info!("Successfully updated booking status to {} for invoice_id {}", status, id);
                    }
                },
                Err(e) => tracing::error!("Database error updating booking status: {}", e),
            }
        }
    }

    StatusCode::OK.into_response()
}

pub async fn test_email() -> impl IntoResponse {
    let to_email = std::env::var("MAIL_TO_ADMIN_ADDRESS").unwrap_or_else(|_| "admin@example.com".to_string());
    let email_template = PaymentSuccessEmail {
        name: "Test User".to_string(),
        plan_name: "DYNASTY".to_string(),
        slug: "test-invitation".to_string(),
        amount: 150000,
        language: "id".to_string(),
        base_url: std::env::var("REDIRECT_APP_BASE_URL").unwrap_or_else(|_| "https://castellant-ai.up.railway.app".to_string()),
    };

    match mailer::send_payment_success_email(&to_email, email_template, None).await {
        Ok(_) => (StatusCode::OK, "Email sent successfully! Check your inbox.").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to send email: {}", e)).into_response(),
    }
}

pub async fn check_slug(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE slug = $1")
        .bind(&slug)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    
    if count > 0 {
        Json(json!({ "available": false, "message": "URL slug already taken" })).into_response()
    } else {
        Json(json!({ "available": true, "message": "URL slug is available" })).into_response()
    }
}

fn parse_event_date_to_iso(date_str: &str) -> String {
    // 1. Handle ISO format YYYY-MM-DD
    if date_str.contains('-') && date_str.len() >= 10 {
        if date_str.len() == 10 {
            return format!("{}T08:00:00", date_str);
        } else if date_str.contains('T') {
            return date_str.to_string();
        }
    }

    // 2. Handle Indonesian format like "12 Desember 2026"
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() == 3 {
        let day = parts[0];
        let month_str = parts[1].to_lowercase();
        let year = parts[2];
        
        let month = match month_str.as_str() {
            "januari" => "01",
            "februari" => "02",
            "maret" => "03",
            "april" => "04",
            "mei" => "05",
            "juni" => "06",
            "juli" => "07",
            "agustus" => "08",
            "september" => "09",
            "oktober" => "10",
            "november" => "11",
            "desember" => "12",
            _ => "05", // Fallback
        };
        
        let day_padded = if day.len() == 1 { format!("0{}", day) } else { day.to_string() };
        return format!("{}-{}-{}T08:00:00", year, month, day_padded);
    }

    "2026-05-24T08:00:00".to_string() // Ultimate fallback
}

fn format_date_for_display(date_str: &str) -> String {
    // If it's already a nice string, keep it (simple heuristic)
    if !date_str.contains('-') || date_str.len() != 10 {
        return date_str.to_string();
    }

    // Handle YYYY-MM-DD
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        let year = parts[0];
        let month_num = parts[1];
        let day = parts[2];
        
        let month_name = match month_num {
            "01" => "Januari",
            "02" => "Februari",
            "03" => "Maret",
            "04" => "April",
            "05" => "Mei",
            "06" => "Juni",
            "07" => "Juli",
            "08" => "Agustus",
            "09" => "September",
            "10" => "Oktober",
            "11" => "November",
            "12" => "Desember",
            _ => return date_str.to_string(),
        };
        
        // Remove leading zero from day
        let day_clean = if day.starts_with('0') { &day[1..] } else { day };
        return format!("{} {} {}", day_clean, month_name, year);
    }

    date_str.to_string()
}
