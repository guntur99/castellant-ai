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
    redirect_url: String,
}

#[derive(Deserialize)]
struct MayarInvoiceResponse {
    #[allow(dead_code)]
    status: bool,
    data: Option<MayarData>,
}

#[derive(Deserialize)]
struct MayarData {
    link: String,
    id: String,
}

#[derive(Template)]
#[template(path = "invitation/create.html")]
pub struct CreateInvitationTemplate {
    pub user: Option<User>,
    #[allow(dead_code)]
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

#[derive(Serialize, Clone)]
pub struct TemplateMetadata {
    pub id: String,
    pub title: String,
    pub desc: String,
    pub category: String,
    pub price: i32,
    pub preview_img: String,
    pub plan: String,
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

pub fn get_all_templates() -> Vec<TemplateMetadata> {
    vec![
        TemplateMetadata {
            id: "trendvibe".to_string(),
            title: "Castellan VibePulse".to_string(),
            desc: "Nikmati pengalaman vertikal eksklusif khas Castellant yang dinamis dan modern, terinspirasi dari tren TikTok untuk momen yang tak terlupakan.".to_string(),
            price: 50000,
            preview_img: "/static/img/trendvibe_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "loveanthem".to_string(),
            title: "Castellan SoulBeat".to_string(),
            desc: "Mainkan melodi cinta Anda dalam antarmuka premium Castellant bergaya Spotify, merayakan perjalanan hati Anda bagaikan lagu hits terpopuler.".to_string(),
            price: 50000,
            preview_img: "/static/img/loveanthem_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "cinemarry".to_string(),
            title: "Castellan CineLove".to_string(),
            desc: "Saksikan keajaiban pernikahan Anda dalam format sinematik Castellant ala Netflix, di mana kisah cinta Anda adalah tayangan utama yang memukau.".to_string(),
            price: 50000,
            preview_img: "/static/img/cinemarry_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "shopee-live-wedding".to_string(),
            title: "Castellan ShopeeLive".to_string(),
            desc: "Bawa keseruan belanja live ke dalam undangan pernikahan Anda dengan antarmuka interaktif khas Shopee Live yang ceria dan penuh energi.".to_string(),
            price: 50000,
            preview_img: "/static/img/shopee-live-wedding_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "tiktok-live-wedding".to_string(),
            title: "Castellan TikTokLive".to_string(),
            desc: "Rasakan sensasi viral dengan tema TikTok Live yang dinamis, memungkinkan tamu Anda berinteraksi layaknya sedang menonton siaran langsung momen bahagia Anda.".to_string(),
            price: 50000,
            preview_img: "/static/img/tiktok-live-wedding_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-uber".to_string(),
            title: "Castellan UberRide".to_string(),
            desc: "Perjalanan cinta yang elegan dan global dengan desain minimalis modern khas Uber, mengantarkan setiap tamu menuju hari istimewa Anda dengan gaya.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-uber_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-disney".to_string(),
            title: "Castellan DisneyMagic".to_string(),
            desc: "Wujudkan dongeng impian Anda dengan sentuhan magis Disney, di mana setiap detail undangan memancarkan keajaiban dan romansa ala kerajaan.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-disney_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-facebook".to_string(),
            title: "Castellan FaceLove".to_string(),
            desc: "Hubungkan kenangan dan kebahagiaan dengan desain yang terinspirasi dari media sosial favorit, menciptakan ruang berbagi cinta yang akrab dan personal.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-facebook_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-iphone-theme".to_string(),
            title: "Castellan iWedding".to_string(),
            desc: "Kemewahan teknologi dalam genggaman dengan antarmuka iOS yang bersih, intuitif, dan sangat premium untuk mempresentasikan kisah cinta modern Anda.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-iphone-theme_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-netflix-v2".to_string(),
            title: "Castellan CineLove Max".to_string(),
            desc: "Edisi terbaru dari pengalaman sinematik Castellant, kini dengan fitur lebih mendalam dan visual yang lebih tajam layaknya tayangan blockbusters terbaik.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-netflix-v2_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-prime".to_string(),
            title: "Castellan PrimeLove".to_string(),
            desc: "Layanan cinta eksklusif dengan estetika premium Amazon Prime, menjanjikan pengiriman kebahagiaan yang cepat, tepat, dan penuh kejutan manis.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-prime_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-wrath-v2".to_string(),
            title: "Castellan EpicWrath".to_string(),
            desc: "Tampilkan kekuatan dan intensitas cinta Anda dalam desain yang dramatis dan megah, terinspirasi dari epik fantasi yang legendaris.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-wrath-v2_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "cairide".to_string(),
            title: "Castellan CaiRide".to_string(),
            desc: "Hadirkan keseruan dalam setiap perjalanan menuju hari bahagia Anda dengan antarmuka dinamis khas Castellant bergaya aplikasi transportasi terpopuler.".to_string(),
            price: 50000,
            preview_img: "/static/img/cairide_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "pinterlove".to_string(),
            title: "Castellan PinterLove".to_string(),
            desc: "Visual estetik dengan tata letak masonry yang memukau, terinspirasi dari Pinterest untuk momen tak terlupakan.".to_string(),
            price: 75000,
            preview_img: "/static/img/pinterlove_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "signature".to_string(),
        },
        TemplateMetadata {
            id: "wedding-applemusic".to_string(),
            title: "Castellan Melody".to_string(),
            desc: "Rayakan harmoni cinta Anda dengan estetika Apple Music yang elegan. Fokus pada album art foto Anda dan lirik cerita cinta yang mengalir.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-applemusic_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-capcut".to_string(),
            title: "Castellan Edit".to_string(),
            desc: "Kisah cinta Anda adalah mahakarya yang sedang diedit. Desain dinamis ala timeline CapCut dengan energi kreatif yang meluap.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-capcut_preview.png".to_string(),
            category: "premium".to_string(),
            plan: "premium".to_string(),
        },
    ]
}

pub async fn templates_list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let templates_data = get_all_templates();
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

    HtmlTemplate(CreateInvitationTemplate { user, is_dev: state.is_dev }).into_response()
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
    let user_id = Uuid::parse_str(&user_id_str).unwrap();

    let mut fields = HashMap::new();
    let mut photo_paths = HashMap::new();
    let mut gallery_paths = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        
        if name == "gallery[]" || name == "gallery_photo" {
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
        } else if name == "payment_proof" {
            let filename = Uuid::new_v4().to_string() + "_payment.jpg";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap();
            if !data.is_empty() {
                std::fs::create_dir_all("static/uploads").unwrap();
                std::fs::write(&path, data).unwrap();
                fields.insert("payment_proof".to_string(), format!("/{}", path));
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

    let plan_name = "essential".to_string(); // Force essential for now
    let amount = 50000;

    let user_row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .unwrap();

    // Call Mayar API
    let mayar_req = MayarInvoiceRequest {
        name: user_row.name.clone().unwrap_or_else(|| "Customer".to_string()),
        email: user_row.email.clone(),
        amount,
        description: format!("Digital Invitation - {} Plan ({})", plan_name.to_uppercase(), fields.get("couple_name_short").unwrap()),
        mobile: "08123456789".to_string(), // Fallback mobile
        redirect_url: format!("{}/invitation/{}", std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()), slug),
    };

    let mayar_res = state.http_client
        .post(&state.mayar_base_url)
        .header("Authorization", format!("Bearer {}", state.mayar_api_key))
        .json(&mayar_req)
        .send()
        .await
        .unwrap()
        .json::<MayarInvoiceResponse>()
        .await
        .unwrap();

    let (payment_link, invoice_id) = if let Some(data) = mayar_res.data {
        (Some(data.link), Some(data.id))
    } else {
        (None, None)
    };

    let template_name = fields.get("template_name").cloned().unwrap_or_else(|| "caiktok".to_string());

    let invitation_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO invitations (user_id, slug, couple_name_short, event_date, template_name, bride_data, groom_data, ceremony_data, reception_data, quote_data, plan_name, payment_link, payment_invoice_id) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id"
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
    .bind(plan_name)
    .bind(&payment_link)
    .bind(invoice_id)
    .fetch_one(&state.db)
    .await
    .unwrap();

    // Insert Gallery Photos
    for (i, path) in gallery_paths.into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO invitation_photos (invitation_id, url, photo_type, \"order\") VALUES ($1, $2, $3, $4)"
        )
        .bind(invitation_id)
        .bind(path)
        .bind("gallery")
        .bind(i as i32)
        .execute(&state.db)
        .await
        .unwrap();
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

    let templates = get_all_templates().into_iter().take(6).collect();

    HtmlTemplate(HomeTemplate { user, invitations, templates, is_dev: state.is_dev }).into_response()
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
                _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
            }
        },
        _ => {
            // Fallback for samples
            if slug.ends_with("-sample") || slug == "sample" {
                let (couple_name, template_name) = match slug.as_str() {
                    "trendvibe-sample" => ("Anita & Zarda", "trendvibe"),
                    "loveanthem-sample" => ("Anita & Zarda", "loveanthem"),
                    "cinemarry-sample" => ("Anita & Zarda", "cinemarry"),
                    "cairide-sample" => ("Anita & Zarda", "cairide"),
                    "pinterlove-sample" => ("Anita & Zarda", "pinterlove"),
                    "shopee-live-wedding-sample" => ("Anita & Zarda", "shopee-live-wedding"),
                    "tiktok-live-wedding-sample" => ("Anita & Zarda", "tiktok-live-wedding"),
                    "we-uber-sample" => ("Anita & Zarda", "we-uber"),
                    "wedding-disney-sample" => ("Anita & Zarda", "wedding-disney"),
                    "wedding-facebook-sample" => ("Anita & Zarda", "wedding-facebook"),
                    "wedding-iphone-theme-sample" => ("Anita & Zarda", "wedding-iphone-theme"),
                    "wedding-netflix-v2-sample" => ("Anita & Zarda", "wedding-netflix-v2"),
                    "wedding-prime-sample" => ("Anita & Zarda", "wedding-prime"),
                    "wedding-wrath-v2-sample" => ("Anita & Zarda", "wedding-wrath-v2"),
                    "wedding-applemusic-sample" => ("Anita & Zarda", "wedding-applemusic"),
                    "we-capcut-sample" => ("Anita & Zarda", "we-capcut"),
                    _ => ("Anita & Zarda", "trendvibe"),
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
                    _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
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
pub async fn preview(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<PreviewRequest>,
) -> impl IntoResponse {
    let invitation = Invitation {
        slug: "preview".to_string(),
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
    };

    match payload.template_name.as_str() {
        "loveanthem" => HtmlTemplate(LoveAnthemTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "cinemarry" => HtmlTemplate(CineMarryTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "cairide" => HtmlTemplate(CaiRideTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "pinterlove" => HtmlTemplate(PinterLoveTemplate { invitation, is_dev: state.is_dev }).into_response(),
        _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
    }
}
