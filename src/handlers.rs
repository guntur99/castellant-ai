use axum::{
    response::{Html, IntoResponse, Response, Redirect},
    http::StatusCode,
    Form,
    extract::{State, Path, Query, Multipart},
    Json,
};
use askama::Template;
use crate::models::{Invitation, Person, EventDetails, Quote, GiftAccount, RsvpForm, InvitationRow, Song, User, AiSession};
use crate::AppState;
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

#[derive(Deserialize)]
pub struct AiGenerateRequest {
    pub prompt: String,
    pub session_id: Option<Uuid>,
    pub context: Option<String>,
}

#[derive(Serialize)]
pub struct AiGenerateResponse {
    pub text: String,
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
            title: "Toktik".to_string(),
            desc: "POV: Wedding kamu masuk fyp. Desain vertikal yang dinamis buat kamu yang selalu up-to-date sama tren kekinian.".to_string(),
            price: 50000,
            preview_img: "/static/img/trendvibe_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "loveanthem".to_string(),
            title: "Spitapy".to_string(),
            desc: "Spotify-inspired interface buat nge-track perjalanan cinta kalian. Definisi 'Our Song' yang dijadiin undangan digital premium.".to_string(),
            price: 50000,
            preview_img: "/static/img/loveanthem_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "cinemarry".to_string(),
            title: "Nitflax".to_string(),
            desc: "Kisah cinta kalian adalah Netflix Original Series terbaik tahun ini. Visual sinematik yang bikin tamu gak sabar buat klik play.".to_string(),
            price: 50000,
            preview_img: "/static/img/cinemarry_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "shopee-live-wedding".to_string(),
            title: "Shoopi".to_string(),
            desc: "Lagi live nih! Undangan interaktif ala Shopee Live buat kamu yang mau tampil beda, ceria, dan penuh energi flash sale kebahagiaan.".to_string(),
            price: 50000,
            preview_img: "/static/img/shopee-live-wedding_preview.png".to_string(),
            category: "e-commerce".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "tiktok-live-wedding".to_string(),
            title: "Toktik Live".to_string(),
            desc: "Sensasi viral TikTok Live di hari pernikahanmu. Tamu bisa tap-tap layar dan kasih gift cinta secara digital di undanganmu.".to_string(),
            price: 50000,
            preview_img: "/static/img/tiktok-live-wedding_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-uber".to_string(),
            title: "Ubar".to_string(),
            desc: "Perjalanan cinta yang clean dan efisien ala Uber interface. Minimalis, modern, dan pastinya gak bakal bikin tamu nyasar.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-uber_preview.png".to_string(),
            category: "on-demand".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-disney".to_string(),
            title: "Disni".to_string(),
            desc: "Main character energy! Wujudkan dongeng impian kamu dengan sentuhan magis yang bikin momen pernikahan kerasa kaya di kerajaan.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-disney_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-facebook".to_string(),
            title: "Pesbuk".to_string(),
            desc: "Bernostalgia dengan interface sosial media yang personal. Bagikan status 'Married' kamu dengan cara yang paling akrab.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-facebook_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-iphone-theme".to_string(),
            title: "iPon".to_string(),
            desc: "Luxury tech vibes. Antarmuka iOS yang clean dan premium buat kamu yang pengen undangan terlihat high-end dan eksklusif.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-iphone-theme_preview.png".to_string(),
            category: "productivity".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-netflix-v2".to_string(),
            title: "Nitflax 2.0".to_string(),
            desc: "Version 2.0 dari seri sinematik kita. Lebih tajam, lebih deep, dan pastinya lebih bikin tamu ketagihan buat scroll sampai habis.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-netflix-v2_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-prime".to_string(),
            title: "Primi".to_string(),
            desc: "Fast delivery of happiness! Estetika premium Amazon Prime yang menjanjikan pengalaman undangan yang sleek dan anti-ribet.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-prime_preview.png".to_string(),
            category: "e-commerce".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-wrath-v2".to_string(),
            title: "Wreth".to_string(),
            desc: "God mode on! Desain dramatis dan megah buat kalian yang pengen momen pernikahannya kerasa kaya epic fantasy legend.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-wrath-v2_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "cairide".to_string(),
            title: "GoJack".to_string(),
            desc: "Otw pelaminan! Antarmuka dinamis aplikasi transportasi yang bikin perjalanan cinta kalian kerasa seru dan penuh petualangan.".to_string(),
            price: 50000,
            preview_img: "/static/img/cairide_preview.png".to_string(),
            category: "on-demand".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "pinterlove".to_string(),
            title: "Pinteres".to_string(),
            desc: "Pinterest-perfect wedding. Tata letak masonry yang memukau buat kamu yang memuja estetika visual di setiap detailnya.".to_string(),
            price: 50000,
            preview_img: "/static/img/pinterlove_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-applemusic".to_string(),
            title: "Apel Musik".to_string(),
            desc: "Love in stereo. Estetika Apple Music yang minimalis, fokus ke album art foto kalian dan lirik cerita cinta yang mengalir syahdu.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-applemusic_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-capcut".to_string(),
            title: "Kepket".to_string(),
            desc: "Trust the process! Desain dinamis ala timeline video editor buat kamu yang ngeliat perjalanan cinta sebagai mahakarya kreatif.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-capcut_preview.png".to_string(),
            category: "productivity".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "bereal-wedding".to_string(),
            title: "BiRil".to_string(),
            desc: "Real moments only. Undangan autentik bergaya BeReal dengan dual kamera, buat kamu yang suka tampil apa adanya dan jujur.".to_string(),
            price: 50000,
            preview_img: "/static/img/bereal-wedding_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "instagram-live-wedding".to_string(),
            title: "Instagrem".to_string(),
            desc: "Rayakan momen spesialmu ala Instagram Live. Interaktif, kekinian, dan bikin tamu merasa benar-benar hadir di sana.".to_string(),
            price: 50000,
            preview_img: "/static/img/instagram-live-wedding_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "qris-wedding".to_string(),
            title: "Kris Love".to_string(),
            desc: "Scan untuk kebahagiaan! Undangan unik dengan tema sistem pembayaran digital yang pastinya bikin tamu senyum-senyum sendiri.".to_string(),
            price: 50000,
            preview_img: "/static/img/qris-wedding_preview.png".to_string(),
            category: "e-commerce".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-grab".to_string(),
            title: "Greb".to_string(),
            desc: "Otw pelaminan dengan gaya! Antarmuka Grab yang familiar untuk memudahkan tamu menemukan lokasi dan detail pernikahanmu.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-grab_preview.png".to_string(),
            category: "on-demand".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "figma-wedding".to_string(),
            title: "Pegma".to_string(),
            desc: "Dibuat dengan presisi pixel-perfect. Untuk pasangan desainer atau tech-enthusiast yang menghargai setiap detail elemen desain.".to_string(),
            price: 50000,
            preview_img: "/static/img/figma-wedding_preview.png".to_string(),
            category: "productivity".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-manga".to_string(),
            title: "Mengu".to_string(),
            desc: "Komik strip cinta kalian dimulai di sini! Estetika panel manga hitam putih yang dramatis dan penuh ekspresi cinta.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-manga_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-nintendo-switch".to_string(),
            title: "Switch Love".to_string(),
            desc: "Level up your wedding! Antarmuka konsol favorit sejuta umat yang interaktif, ceria, dan penuh warna kegembiraan.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-nintendo-switch_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-kai-v2".to_string(),
            title: "KAI Access 2.0".to_string(),
            desc: "Tiket menuju masa depan bersama! Versi upgrade dari tema kereta api yang lebih modern, detail, dan pastinya on-time.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-kai-v2_preview.png".to_string(),
            category: "on-demand".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-minecraft".to_string(),
            title: "Maincreft".to_string(),
            desc: "Membangun rumah tangga blok demi blok. Estetika pixel art yang ikonik buat pasangan gamer yang kreatif dan adventurous.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-minecraft_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-zoom-v2".to_string(),
            title: "Zoomy 2.0".to_string(),
            desc: "You're not on mute! Versi terbaru tema video call yang lebih interaktif, bikin semua tamu merasa di satu meeting yang sama.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-zoom-v2_preview.png".to_string(),
            category: "productivity".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-vscode".to_string(),
            title: "Code Love".to_string(),
            desc: "Commit to a lifetime of happiness! Antarmuka VS Code yang ikonik buat pasangan developer yang pengen undangannya terlihat geeky namun tetap premium.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-vscode_preview.png".to_string(),
            category: "productivity".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-discord".to_string(),
            title: "Diskort".to_string(),
            desc: "Join the server! Undangan bertema Discord untuk komunitas gamer atau tech-enthusiast yang ingin merayakan cinta dalam mode 'Online'.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-discord_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-webtoon".to_string(),
            title: "Wibton".to_string(),
            desc: "Baca kisah cinta kalian episode demi episode. Format vertikal ala komik digital yang unik dan menarik untuk diikuti.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-webtoon_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-whatsapp-theme".to_string(),
            title: "Watsap".to_string(),
            desc: "Dari chat jadi akad. Interface WhatsApp yang sangat akrab untuk membagikan kabar bahagia kalian secara personal.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-whatsapp-theme_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-mixue".to_string(),
            title: "Miksu".to_string(),
            desc: "Manisnya cinta ala Mixue! Undangan ceria dengan maskot Snowman yang bikin suasana pernikahan jadi makin 'fresh' dan 'sweet'.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-mixue_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-playstation".to_string(),
            title: "Plesetesi".to_string(),
            desc: "Achievement Unlocked: Married! Tema PlayStation untuk pasangan gamers yang siap memulai petualangan baru di level kehidupan selanjutnya.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-playstation_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "we-threads-app".to_string(),
            title: "Treds".to_string(),
            desc: "Utas cinta yang tak terputus. Desain minimalis ala Threads untuk kamu yang ingin membagikan momen sakral dalam format teks yang intim.".to_string(),
            price: 50000,
            preview_img: "/static/img/we-threads-app_preview.png".to_string(),
            category: "social".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-alfamart".to_string(),
            title: "Alpamart".to_string(),
            desc: "Belanja kebahagiaan di sini! Undangan unik bertema aplikasi minimarket favorit Indonesia, lengkap dengan poin cinta tanpa batas.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-alfamart_preview.png".to_string(),
            category: "e-commerce".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-kai".to_string(),
            title: "KAI Akss".to_string(),
            desc: "Tiket menuju kebahagiaan. Antarmuka KAI Access yang familiar untuk mengantar tamu ke stasiun pelaminan kalian tepat waktu.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-kai_preview.png".to_string(),
            category: "on-demand".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-medium".to_string(),
            title: "Medyum".to_string(),
            desc: "Tuliskan narasi cinta kalian dalam format artikel Medium yang elegan. Untuk pasangan yang punya banyak cerita indah untuk dibagikan.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-medium_preview.png".to_string(),
            category: "entertainment".to_string(),
            plan: "premium".to_string(),
        },
        TemplateMetadata {
            id: "wedding-transjakarta".to_string(),
            title: "TransJak".to_string(),
            desc: "Pemberhentian terakhir: Pelaminan! Tema TransJakarta yang ikonik untuk memandu tamu menyusuri rute kebahagiaan kalian.".to_string(),
            price: 50000,
            preview_img: "/static/img/wedding-transjakarta_preview.png".to_string(),
            category: "on-demand".to_string(),
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
        redirect_url: format!("{}/invitation/{}/manage", std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()), slug),
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
#[template(path = "invitation/manage.html")]
pub struct ManageInvitationTemplate {
    pub invitation: Invitation,
    pub all_templates: Vec<TemplateMetadata>,
    pub is_dev: bool,
    pub user: Option<User>,
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
                slug: row.slug.clone(),
                template_name: row.template_name.clone(),
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
                _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
            }
        },
        _ => {
            // Fallback for samples
            if slug.ends_with("-sample") || slug == "sample" {
                let (couple_name, template_name) = match slug.as_str() {
                    "trendvibe-sample" => ("Nazma & Guntur", "trendvibe"),
                    "loveanthem-sample" => ("Nazma & Guntur", "loveanthem"),
                    "cinemarry-sample" => ("Nazma & Guntur", "cinemarry"),
                    "cairide-sample" => ("Nazma & Guntur", "cairide"),
                    "pinterlove-sample" => ("Nazma & Guntur", "pinterlove"),
                    "shopee-live-wedding-sample" => ("Nazma & Guntur", "shopee-live-wedding"),
                    "tiktok-live-wedding-sample" => ("Nazma & Guntur", "tiktok-live-wedding"),
                    "we-uber-sample" => ("Nazma & Guntur", "we-uber"),
                    "wedding-disney-sample" => ("Nazma & Guntur", "wedding-disney"),
                    "wedding-facebook-sample" => ("Nazma & Guntur", "wedding-facebook"),
                    "wedding-iphone-theme-sample" => ("Nazma & Guntur", "wedding-iphone-theme"),
                    "wedding-netflix-v2-sample" => ("Nazma & Guntur", "wedding-netflix-v2"),
                    "wedding-prime-sample" => ("Nazma & Guntur", "wedding-prime"),
                    "wedding-wrath-v2-sample" => ("Nazma & Guntur", "wedding-wrath-v2"),
                    "wedding-applemusic-sample" => ("Nazma & Guntur", "wedding-applemusic"),
                    "we-capcut-sample" => ("Nazma & Guntur", "we-capcut"),
                    "bereal-wedding-sample" => ("Nazma & Guntur", "bereal-wedding"),
                    "instagram-live-wedding-sample" => ("Nazma & Guntur", "instagram-live-wedding"),
                    "qris-wedding-sample" => ("Nazma & Guntur", "qris-wedding"),
                    "wedding-grab-sample" => ("Nazma & Guntur", "wedding-grab"),
                    "figma-wedding-sample" => ("Nazma & Guntur", "figma-wedding"),
                    "we-discord-sample" => ("Nazma & Guntur", "we-discord"),
                    "we-webtoon-sample" => ("Nazma & Guntur", "we-webtoon"),
                    "we-mixue-sample" => ("Nazma & Guntur", "we-mixue"),
                    "we-playstation-sample" => ("Nazma & Guntur", "we-playstation"),
                    "we-threads-app-sample" => ("Nazma & Guntur", "we-threads-app"),
                    "wedding-alfamart-sample" => ("Nazma & Guntur", "wedding-alfamart"),
                    "wedding-kai-sample" => ("Nazma & Guntur", "wedding-kai"),
                    "wedding-medium-sample" => ("Nazma & Guntur", "wedding-medium"),
                    "wedding-transjakarta-sample" => ("Nazma & Guntur", "wedding-transjakarta"),
                    "wedding-whatsapp-theme-sample" => ("Nazma & Guntur", "wedding-whatsapp-theme"),
                    "we-manga-sample" => ("Nazma & Guntur", "we-manga"),
                    "we-nintendo-switch-sample" => ("Nazma & Guntur", "we-nintendo-switch"),
                    "wedding-kai-v2-sample" => ("Nazma & Guntur", "wedding-kai-v2"),
                    "wedding-minecraft-sample" => ("Nazma & Guntur", "wedding-minecraft"),
                    "wedding-zoom-v2-sample" => ("Nazma & Guntur", "wedding-zoom-v2"),
                    "we-vscode-sample" => ("Nazma & Guntur", "we-vscode"),
                    _ => ("Nazma & Guntur", "trendvibe"),
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
                gift_accounts: Vec::new(),
                song_url: String::new(),
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

            let all_templates = get_all_templates();
            HtmlTemplate(ManageInvitationTemplate { 
                invitation, 
                all_templates, 
                is_dev: state.is_dev,
                user,
            }).into_response()
        },
        None => (StatusCode::NOT_FOUND, "Invitation not found or unauthorized").into_response(),
    }
}

pub async fn update_invitation(
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

    let mut row = row.unwrap();
    
    if let Some(val) = fields.get("couple_name_short") {
        row.couple_name_short = val.clone();
    }
    if let Some(val) = fields.get("event_date") {
        row.event_date = val.clone();
    }
    
    // Update JSON Data
    let mut bride: Person = from_value(row.bride_data).unwrap();
    if let Some(val) = fields.get("bride_name") { bride.name = val.clone(); }
    if let Some(val) = fields.get("bride_full_name") { bride.full_name = val.clone(); }

    let mut groom: Person = from_value(row.groom_data).unwrap();
    if let Some(val) = fields.get("groom_name") { groom.name = val.clone(); }
    if let Some(val) = fields.get("groom_full_name") { groom.full_name = val.clone(); }

    sqlx::query(
        "UPDATE invitations SET couple_name_short = $1, event_date = $2, bride_data = $3, groom_data = $4 WHERE id = $5"
    )
    .bind(&row.couple_name_short)
    .bind(&row.event_date)
    .bind(json!(bride))
    .bind(json!(groom))
    .bind(row.id)
    .execute(&state.db)
    .await
    .unwrap();

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
            
            Json(AiGenerateResponse { text }).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("AI request failed: {}", e)).into_response()
        }
    }
}

pub async fn ai_guest_chat(
    State(state): State<AppState>,
    Json(payload): Json<AiGenerateRequest>,
) -> impl IntoResponse {
    let api_key = &state.sumopod_api_key;
    let base_url = &state.sumopod_base_url;
    let model = &state.sumopod_model;

    if api_key.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "AI API Key not configured").into_response();
    }

    let invitation_context = payload.context.unwrap_or_else(|| "No wedding details provided.".to_string());

    let messages = json!([
        {
            "role": "system",
            "content": format!(
                "You are a helpful Wedding Concierge. Use the following wedding details to answer guest questions. 
                Be polite, warm, and helpful. If you don't know the answer, politely ask them to contact the couple directly.
                
                WEDDING DETAILS:
                {}", 
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
            
            Json(AiGenerateResponse { text }).into_response()
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
            3. Return a JSON object with:
               - 'data': The extracted fields (merged with current state).
               - 'missing': A list of ALL fields that are still empty.
               - 'reply': A conversational reply in Indonesian. 
                 - ONLY ask for 2-4 missing fields per turn.
                 - Prioritize critical fields (Names, Date, Venue).
                 - Use a friendly 'korek info' tone.
                 - ALWAYS remind them about media (Gallery/Video) when text fields are nearly complete.
            4. Use YYYY-MM-DD for dates.
            5. ONLY return the JSON object.", current_form.to_string())
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
            let ai_text = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("{}")
                .to_string();
            
            let parsed_ai: serde_json::Value = serde_json::from_str(&ai_text).unwrap_or_default();
            let ai_reply = parsed_ai["reply"].as_str().unwrap_or("Done!").to_string();
            
            history.push(serde_json::json!({ "role": "assistant", "content": ai_reply }));
            
            let _ = sqlx::query("UPDATE ai_sessions SET chat_history = $1, form_state = $2, updated_at = NOW() WHERE id = $3")
                .bind(serde_json::to_value(&history).unwrap_or_default())
                .bind(&parsed_ai["data"])
                .bind(session_id)
                .execute(&state.db)
                .await;

            let response_text = serde_json::json!({
                "session_id": session_id,
                "text": ai_text
            }).to_string();

            axum::Json(AiGenerateResponse { text: response_text }).into_response()
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
