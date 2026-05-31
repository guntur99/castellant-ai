use axum::{
    response::{Html, IntoResponse, Response, Redirect},
    http::StatusCode,
    Form,
    extract::{State, Path, Query, Multipart},
    Json,
};
use askama::Template;
use crate::models::{Invitation, Person, EventDetails, Quote, GiftAccount, RsvpForm, Rsvp, Story, InvitationRow, Song, User, AiSession, Guest, GuestGroup, Booking, Voucher, InvitationTemplate, Plan, Referral, ReferralHistory, BlogPost};
use crate::AppState;
mod filters {
    pub use crate::filters::*;
}

use crate::mailer::{self, PaymentSuccessEmail};
use chrono::Utc;
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
    pub items: Vec<MayarItem>,
    #[serde(rename = "extraData")]
    pub extra_data: HashMap<String, String>,
}

async fn process_and_save_image(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    data: axum::body::Bytes,
    save_path: String,
    max_dim: u32,
) -> bool {
    tracing::info!("Processing image for path: {}", save_path);
    // 1. Process image in blocking task (CPU intensive)
    let processed_data = tokio::task::spawn_blocking(move || {
        match image::load_from_memory(&data) {
            Ok(img) => {
                let scaled = if img.width() > max_dim || img.height() > max_dim {
                    img.resize(max_dim, max_dim, image::imageops::FilterType::Lanczos3)
                } else {
                    img
                };
                
                let mut buffer = std::io::Cursor::new(Vec::new());
                if let Err(e) = scaled.write_to(&mut buffer, image::ImageFormat::WebP) {
                    tracing::error!("Failed to write WebP: {}", e);
                    return None;
                }
                Some(buffer.into_inner())
            },
            Err(e) => {
                tracing::error!("Failed to load image from memory: {}", e);
                None
            }
        }
    }).await.unwrap_or(None);

    // 2. Upload to S3 if processed successfully
    if let Some(bytes) = processed_data {
        let body = aws_sdk_s3::primitives::ByteStream::from(bytes);
        let key = save_path.trim_start_matches('/');
        
        let res = s3
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(body)
            .content_type("image/webp")
            .cache_control("public, max-age=31536000")
            .acl(aws_sdk_s3::types::ObjectCannedAcl::PublicRead)
            .send()
            .await;
            
        if let Err(e) = &res {
            tracing::error!("Failed to upload to S3: {}", e);
        } else {
            tracing::info!("Successfully uploaded to S3: {}", key);
        }
        return res.is_ok();
    }
    
    tracing::error!("Image processing failed for path: {}", save_path);
    false
}

async fn save_raw_file(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    data: axum::body::Bytes,
    save_path: String,
    content_type: &str,
) -> bool {
    tracing::info!("Uploading raw file for path: {}", save_path);
    let body = aws_sdk_s3::primitives::ByteStream::from(data);
    let key = save_path.trim_start_matches('/');
    
    let res = s3
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .content_type(content_type)
        .cache_control("public, max-age=31536000")
        .acl(aws_sdk_s3::types::ObjectCannedAcl::PublicRead)
        .send()
        .await;
        
    if let Err(e) = &res {
        tracing::error!("Failed to upload raw file to S3: {}", e);
    } else {
        tracing::info!("Successfully uploaded raw file to S3: {}", key);
    }
    res.is_ok()
}

async fn compress_and_save_video(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    data: axum::body::Bytes,
    save_path: String,
) -> Option<String> {
    let temp_uuid = uuid::Uuid::new_v4().to_string();
    let temp_dir = std::env::current_dir().unwrap_or_default().join("scratch");
    let input_path = temp_dir.join(format!("in_{}", temp_uuid));
    let output_path = temp_dir.join(format!("out_{}.webm", temp_uuid));

    // Fail-safe default: Upload raw original file if FFmpeg fails or is missing
    let upload_raw = || async {
        tracing::warn!("FFmpeg not available or failed. Uploading raw video file directly.");
        if save_raw_file(s3, bucket, data.clone(), save_path.clone(), "video/mp4").await {
            if save_path.starts_with("static/uploads/") {
                return Some(format!("/uploads/{}", &save_path["static/uploads/".len()..]));
            }
            return Some(save_path.clone());
        }
        None
    };

    if tokio::fs::write(&input_path, &data).await.is_err() {
        return upload_raw().await;
    }

    // Run FFmpeg: Convert to WebM, resize width to 640px, strip audio (-an), constant quality CRF 32
    let ffmpeg_status = tokio::process::Command::new("ffmpeg")
        .args(&[
            "-i", input_path.to_str().unwrap_or(""),
            "-vf", "scale=640:-2",
            "-c:v", "libvpx-vp9",
            "-crf", "32",
            "-b:v", "0",
            "-an",
            "-y",
            output_path.to_str().unwrap_or(""),
        ])
        .status()
        .await;

    let mut final_path = None;
    if let Ok(status) = ffmpeg_status {
        if status.success() {
            if let Ok(compressed_bytes) = tokio::fs::read(&output_path).await {
                let webm_path = save_path.replace(".mp4", ".webm");
                if save_raw_file(
                    s3,
                    bucket,
                    axum::body::Bytes::from(compressed_bytes),
                    webm_path.clone(),
                    "video/webm",
                ).await {
                    if webm_path.starts_with("static/uploads/") {
                        final_path = Some(format!("/uploads/{}", &webm_path["static/uploads/".len()..]));
                    } else {
                        final_path = Some(webm_path);
                    }
                }
            }
        }
    }

    if final_path.is_none() {
        // Fallback to raw upload if FFmpeg failed
        final_path = upload_raw().await;
    }

    // Cleanup temp files
    let _ = tokio::fs::remove_file(&input_path).await;
    let _ = tokio::fs::remove_file(&output_path).await;

    final_path
}

async fn delete_s3_file(s3: &aws_sdk_s3::Client, bucket: &str, url: &str) {
    if url.is_empty() || !url.starts_with("/uploads/") {
        return;
    }
    
    // Convert /uploads/... to static/uploads/...
    let key = format!("static{}", url);
    let key = key.trim_start_matches('/');
    
    tracing::info!("Deleting file from S3: {}", key);
    let res = s3.delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await;
        
    if let Err(e) = res {
        tracing::error!("Failed to delete from S3: {}", e);
    }
}

pub async fn process_and_save_file(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    data: axum::body::Bytes,
    save_path: String,
    content_type: &str,
) -> bool {
    tracing::info!("Uploading file to path: {} (type: {})", save_path, content_type);
    
    let body = aws_sdk_s3::primitives::ByteStream::from(data);
    let key = save_path.trim_start_matches('/');
    
    let res = s3
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .content_type(content_type)
        .cache_control("public, max-age=31536000")
        .acl(aws_sdk_s3::types::ObjectCannedAcl::PublicRead)
        .send()
        .await;
        
    if let Err(e) = &res {
        tracing::error!("Failed to upload file to S3: {}", e);
    } else {
        tracing::info!("Successfully uploaded file to S3: {}", key);
    }
    res.is_ok()
}

pub fn rewrite_s3_url_to_proxy(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        if url.contains("storageapi.dev") || url.contains("amazonaws.com") || url.contains("supabase.co") {
            let stripped = if url.starts_with("https://") {
                &url["https://".len()..]
            } else {
                &url["http://".len()..]
            };
            if let Some(pos) = stripped.find('/') {
                let path = &stripped[pos + 1..];
                if path.starts_with("static/uploads/") {
                    return format!("/uploads/{}", &path["static/uploads/".len()..]);
                }
                return format!("/uploads/{}", path);
            }
        }
    }
    url.to_string()
}

pub fn sanitize_template_urls(mut t: InvitationTemplate) -> InvitationTemplate {
    t.preview_img = rewrite_s3_url_to_proxy(&t.preview_img);
    if let Some(ref video_url) = t.preview_video {
        t.preview_video = Some(rewrite_s3_url_to_proxy(video_url));
    }
    t
}

pub async fn serve_upload(
    Path(key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut candidate_keys = vec![
        format!("static/uploads/{}", key),
    ];
    if key.starts_with("static/") {
        candidate_keys.push(key.clone());
    } else {
        candidate_keys.push(format!("static/{}", key));
        candidate_keys.push(key.clone());
    }

    let mut response_data = None;
    for s3_key in candidate_keys {
        match state.s3_client
            .get_object()
            .bucket(&state.s3_bucket)
            .key(&s3_key)
            .send()
            .await
        {
            Ok(resp) => {
                response_data = Some((s3_key, resp));
                break;
            }
            Err(e) => {
                tracing::debug!("Failed to find key '{}' in S3: {:?}", s3_key, e);
            }
        }
    }

    match response_data {
        Some((found_key, resp)) => {
            let extension = found_key.split('.').last().unwrap_or("");
            let content_type = match extension {
                "mp3" => "audio/mpeg",
                "mp4" => "video/mp4",
                "webm" => "video/webm",
                "webp" => "image/webp",
                _ => resp.content_type().unwrap_or("application/octet-stream"),
            }.to_string();

            let body_bytes = resp.body.collect().await
                .map(|b| b.into_bytes())
                .unwrap_or_default();
            
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", content_type)
                .header("Cache-Control", "public, max-age=31536000, immutable")
                .body(axum::body::Body::from(body_bytes))
                .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response())
        },
        None => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
    }
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
    pub all_templates: Vec<InvitationTemplate>,
    pub plans: Vec<Plan>,
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


#[derive(Template)]
#[template(path = "invitation/templates_list.html")]
pub struct TemplatesListTemplate {
    pub user: Option<User>,
    pub active_category: String,
    pub templates: Vec<InvitationTemplate>,
    pub current_page: i32,
    pub total_pages: i32,
    pub search_query: String,
    pub sort: String,
    pub is_dev: bool,
}

pub async fn get_all_templates(db: &sqlx::PgPool, only_published: bool) -> Vec<InvitationTemplate> {
    let query = if only_published {
        "SELECT * FROM templates WHERE status = 'PUBLISHED' ORDER BY created_at DESC"
    } else {
        "SELECT * FROM templates ORDER BY created_at DESC"
    };
    let templates = sqlx::query_as::<_, InvitationTemplate>(query)
        .fetch_all(db)
        .await
        .unwrap_or_default();
    
    templates.into_iter().map(sanitize_template_urls).collect()
}

pub async fn get_paginated_templates(
    db: &sqlx::PgPool,
    params: &HashMap<String, String>,
) -> (String, Vec<InvitationTemplate>, i32, i32, String, String) {
    let templates_data = get_all_templates(db, true).await;
    let category = params.get("category").cloned().unwrap_or_else(|| "all".to_string());
    let search = params.get("search").cloned().unwrap_or_default().to_lowercase();
    let sort = params.get("sort").cloned().unwrap_or_else(|| "featured".to_string());
    let page = params.get("page").and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
    let per_page = 6;

    let mut filtered: Vec<InvitationTemplate> = templates_data.into_iter()
        .filter(|t| category == "all" || t.category == category)
        .filter(|t| search.is_empty() || t.title.to_lowercase().contains(&search) || t.desc.to_lowercase().contains(&search))
        .collect();

    // Apply Sorting
    match sort.as_str() {
        "newest" => filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        "oldest" => filtered.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
        "title_asc" => filtered.sort_by(|a, b| a.title.cmp(&b.title)),
        "featured" => filtered.sort_by(|a, b| {
            // Featured first, then newest
            match b.is_featured.cmp(&a.is_featured) {
                std::cmp::Ordering::Equal => b.created_at.cmp(&a.created_at),
                other => other,
            }
        }),
        _ => filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
    }

    let total_pages = ((filtered.len() as f32) / (per_page as f32)).ceil() as i32;
    let start_idx = ((page - 1) * per_page) as usize;
    let paginated = filtered.into_iter()
        .skip(start_idx)
        .take(per_page as usize)
        .collect();

    (category, paginated, page, total_pages, search, sort)
}

pub async fn templates_list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
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
            } else {
                None
            }
        }
        None => None,
    };

    let (category, paginated, page, total_pages, search, sort) = get_paginated_templates(&state.db, &params).await;

    HtmlTemplate(TemplatesListTemplate { 
        user, 
        active_category: category,
        templates: paginated,
        current_page: page,
        total_pages,
        search_query: search,
        sort,
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
    
    let plans = sqlx::query_as::<_, Plan>("SELECT * FROM plans WHERE is_active = true ORDER BY price ASC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    HtmlTemplate(CreateInvitationTemplate { 
        user, 
        all_templates,
        plans,
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

    let mut playlist_paths = Vec::new();
    let mut background_video_url = None;
    let mut gallery_video_paths = Vec::new();
    let mut story_titles = Vec::new();
    let mut story_dates = Vec::new();
    let mut story_descriptions = Vec::new();
    let mut story_image_urls = Vec::new();
    let mut story_image_files = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = match field.name() {
            Some(n) => n.to_string(),
            None => continue,
        };
        
        if name == "gallery[]" || name == "gallery_photo" {
            let filename = Uuid::new_v4().to_string() + ".webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1600).await {
                    gallery_paths.push(format!("/uploads/{}", filename));
                }
            }
        } else if name.ends_with("_photo") {
            let filename = Uuid::new_v4().to_string() + ".webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1600).await {
                    photo_paths.insert(name, format!("/uploads/{}", filename));
                }
            }
        } else if name == "payment_proof" {
            let filename = Uuid::new_v4().to_string() + "_payment.webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1600).await {
                    fields.insert("payment_proof".to_string(), format!("/uploads/{}", filename));
                }
            }
        } else if name == "bank_name[]" {
            bank_names.push(field.text().await.unwrap_or_default());
        } else if name == "account_number[]" {
            account_numbers.push(field.text().await.unwrap_or_default());
        } else if name == "account_holder[]" {
            account_holders.push(field.text().await.unwrap_or_default());
        } else if name == "playlist[]" {
            let filename = Uuid::new_v4().to_string() + ".mp3";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() && data.len() < 10 * 1024 * 1024 {
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "audio/mpeg").await {
                    playlist_paths.push(format!("/uploads/{}", filename));
                }
            }
        } else if name == "background_video" {
            let filename = Uuid::new_v4().to_string() + ".mp4";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() && data.len() < 25 * 1024 * 1024 {
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "video/mp4").await {
                    background_video_url = Some(format!("/uploads/{}", filename));
                }
            }
        } else if name == "gallery_video[]" {
            let filename = Uuid::new_v4().to_string() + ".mp4";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() && data.len() < 25 * 1024 * 1024 {
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "video/mp4").await {
                    gallery_video_paths.push(format!("/uploads/{}", filename));
                }
            }
        } else if name == "story_title[]" {
            story_titles.push(field.text().await.unwrap_or_default());
        } else if name == "story_date[]" {
            story_dates.push(field.text().await.unwrap_or_default());
        } else if name == "story_description[]" {
            story_descriptions.push(field.text().await.unwrap_or_default());
        } else if name == "story_image_url[]" {
            story_image_urls.push(field.text().await.unwrap_or_default());
        } else if name == "story_image_file[]" {
            let filename = Uuid::new_v4().to_string() + ".webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1200).await {
                    story_image_files.push(Some(format!("/uploads/{}", filename)));
                } else {
                    story_image_files.push(None);
                }
            } else {
                story_image_files.push(None);
            }
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
        enabled: true,
        date: fields.get("event_date").cloned().unwrap_or_default(),
        time: fields.get("ceremony_time").cloned().unwrap_or_else(|| "09:00 - selesai".to_string()),
        venue: fields.get("ceremony_venue").cloned().unwrap_or_default(),
        address: fields.get("ceremony_address").cloned().unwrap_or_default(),
        maps_url: fields.get("ceremony_maps").cloned().unwrap_or_default(),
    });

    let reception_data = json!(EventDetails {
        enabled: true,
        date: fields.get("reception_date").cloned().unwrap_or_else(|| fields.get("event_date").cloned().unwrap_or_default()),
        time: fields.get("reception_time").cloned().unwrap_or_else(|| "11:00 - selesai".to_string()),
        venue: fields.get("reception_venue").cloned().unwrap_or_default(),
        address: fields.get("reception_address").cloned().unwrap_or_default(),
        maps_url: fields.get("reception_maps").cloned().unwrap_or_default(),
    });

    let quote_data = json!({
        "text": fields.get("quote_text").cloned().unwrap_or_else(|| "Dan di antara tanda-tanda kekuasaan-Nya ialah Dia menciptakan untukmu isteri-isteri dari jenismu sendiri, supaya kamu cenderung dan merasa tenteram kepadanya, dan dijadikan-Nya diantaramu rasa kasih dan sayang. Sesungguhnya pada yang demikian itu benar-benar terdapat tanda-tanda bagi kaum yang berfikir.".to_string()),
        "source": fields.get("quote_source").cloned().unwrap_or_else(|| "QS. Ar-Rum: 21".to_string())
    });

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

    let mut plan_name = fields.get("plan_name").cloned().unwrap_or_else(|| "NOBLE".to_string());
    
    // Superadmin is always ROYAL
    if user_row.role == "SUPERADMIN" {
        plan_name = "ROYAL".to_string();
    }

    let db_plan = sqlx::query_as::<_, Plan>("SELECT * FROM plans WHERE code = $1")
        .bind(&plan_name)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let amount = match db_plan {
        Some(p) => p.price,
        None => 50000, // Fallback
    };

    let mut discount_amount = 0;
    let mut applied_voucher = None;
    let promo_code = fields.get("promo_code").cloned().unwrap_or_default();

    if !promo_code.is_empty() {
        // Check referrals first
        let referral = sqlx::query_as::<_, Referral>("SELECT * FROM referrals WHERE code = $1 AND is_active = true")
            .bind(&promo_code)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);
            
        if let Some(r) = referral {
            discount_amount = (amount * r.discount_percent) / 100;
            applied_voucher = Some(r.code);
        } else {
            // Check vouchers
            let voucher = sqlx::query_as::<_, Voucher>("SELECT * FROM vouchers WHERE code = $1 AND is_active = true AND (usage_limit IS NULL OR usage_count < usage_limit) AND (valid_until IS NULL OR valid_until > NOW())")
                .bind(&promo_code)
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

    let mut extra_data = HashMap::new();
    extra_data.insert("invitation_slug".to_string(), slug.clone());
    extra_data.insert("target_plan".to_string(), plan_name.clone());
    if let Some(code) = &applied_voucher {
        extra_data.insert("voucher_code".to_string(), code.clone());
    }

    let (payment_link, invoice_id) = if user_row.role == "SUPERADMIN" {
        // Skip Mayar for Superadmin
        (Some(format!("{}/invitation/{}/manage", std::env::var("REDIRECT_APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()), slug)), Some("SUPERADMIN-BYPASS".to_string()))
    } else {
        let items = vec![MayarItem {
            quantity: 1,
            rate: final_amount,
            description: format!("Digital Invitation - {} Plan", plan_name.to_uppercase()),
        }];

        // Call Mayar API
        let mayar_req = MayarInvoiceRequest {
            name: user_row.name.clone().unwrap_or_else(|| "Customer".to_string()),
            email: user_row.email.clone(),
            amount: final_amount,
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
                if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                    if let Some(msg) = err_json.get("messages").and_then(|m| m.as_str()) {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Payment Gateway Error: {}", msg)).into_response();
                    }
                }
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to process payment response").into_response();
            }
        };

        if !mayar_res.status || mayar_res.status_code >= 400 {
            let error_msg = mayar_res.messages.clone().unwrap_or_else(|| "Unknown payment gateway error".to_string());
            eprintln!("Mayar API Error: {}", error_msg);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Payment Gateway Error: {}", error_msg)).into_response();
        }

        let (link, id) = if let Some(data) = mayar_res.data {
            let l = data.get("link").and_then(|l| l.as_str().map(|s| s.to_string()));
            let mut rid = data.get("id").and_then(|i| i.as_str().map(|s| s.to_string()));
            
            if let Some(ref link_str) = l {
                if let Some(readable_id) = link_str.split('/').last() {
                    rid = Some(readable_id.to_string());
                }
            }
            (l, rid)
        } else {
            (None, None)
        };
        (link, id)
    };

    let template_name = fields.get("template_name").cloned().unwrap_or_else(|| "caiktok".to_string());
    let language = fields.get("language").cloned().unwrap_or_else(|| "id".to_string());
    let couple_name_short = fields.get("couple_name_short").cloned().unwrap_or_else(|| "Couple".to_string());
    let event_date = fields.get("event_date").cloned().unwrap_or_else(|| "TBA".to_string());

    // Process Stories
    let mut final_stories = Vec::new();
    let story_count = story_titles.len();
    for i in 0..story_count {
        let title = story_titles.get(i).cloned().unwrap_or_default();
        let date = story_dates.get(i).cloned().unwrap_or_default();
        let description = story_descriptions.get(i).cloned().unwrap_or_default();
        
        let image_url = if let Some(Some(new_path)) = story_image_files.get(i) {
            new_path.clone()
        } else {
            story_image_urls.get(i).cloned().unwrap_or_default()
        };

        if !title.is_empty() {
            final_stories.push(Story {
                id: Uuid::new_v4().to_string(),
                title,
                date,
                description,
                image_url,
            });
        }
    }

    // Process Playlist limits
    let song_limit = match plan_name.as_str() {
        "DYNASTY" => 5,
        "ROYAL" => 3,
        _ => 1,
    };
    let mut final_playlist = playlist_paths;
    final_playlist.truncate(song_limit);

    // Get hero video vertical position
    let hero_video_position = fields.get("hero_video_position")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(50);

    // START TRANSACTION
    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let invitation_id = match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO invitations (user_id, slug, couple_name_short, event_date, template_name, bride_data, groom_data, ceremony_data, reception_data, quote_data, plan_name, language, payment_link, payment_invoice_id, playlist, background_video_url, hero_video_position, stories_data) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) RETURNING id"
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
    .bind(json!(final_playlist))
    .bind(background_video_url)
    .bind(hero_video_position)
    .bind(json!(final_stories))
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

    // Insert Gallery Videos
    for (i, path) in gallery_video_paths.into_iter().enumerate() {
        if let Err(e) = sqlx::query(
            "INSERT INTO invitation_photos (invitation_id, url, photo_type, \"order\") VALUES ($1, $2, $3, $4)"
        )
        .bind(invitation_id)
        .bind(path)
        .bind("gallery_video")
        .bind(i as i32)
        .execute(&mut *tx)
        .await {
            let _ = tx.rollback().await;
            eprintln!("Failed to insert gallery video: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save gallery videos").into_response();
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
            "INSERT INTO bookings (user_id, invitation_id, target_plan, amount, invoice_id, payment_link, status, voucher_code, discount_amount) 
             VALUES ($1, $2, $3, $4, $5, $6, 'PENDING', $7, $8)"
        )
        .bind(user_id)
        .bind(invitation_id)
        .bind(&plan_name)
        .bind(final_amount)
        .bind(inv_id)
        .bind(payment_link.clone())
        .bind(&applied_voucher)
        .bind(discount_amount)
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

pub async fn delete_invitation(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user_id = match jar.get("user_id") {
        Some(c) => Uuid::parse_str(c.value()).ok(),
        None => None,
    };

    if user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    // Soft delete by setting deleted_at and renaming slug to free it up
    let new_slug = format!("deleted-{}-{}", slug, Utc::now().timestamp());
    
    let res = sqlx::query(
        "UPDATE invitations SET deleted_at = NOW(), slug = $1 WHERE slug = $2 AND user_id = $3 AND deleted_at IS NULL"
    )
    .bind(new_slug)
    .bind(&slug)
    .bind(user_id.unwrap())
    .execute(&state.db)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => Redirect::to("/dashboard").into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, "Invitation not found or already deleted").into_response(),
        Err(e) => {
            eprintln!("Failed to delete invitation: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
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
    .bind("dev@castellant.com")
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
    let jar = jar.remove(Cookie::build(("user_id", "")).path("/").build());
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
    pub templates: Vec<InvitationTemplate>,
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
#[template(path = "invitation/super-wedbros.html")]
pub struct SuperWedbrosTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/keraton-dark-invitation.html")]
pub struct KeratonDarkInvitationTemplate {
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
#[template(path = "invitation/royal-heritage.html")]
pub struct RoyalHeritageTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/high-fashion-editorial.html")]
pub struct HighFashionEditorialTemplate {
    #[allow(dead_code)]
    pub invitation: Invitation,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "invitation/reel-wedding.html")]
pub struct ReelWeddingTemplate {
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
#[allow(dead_code)]
pub struct ManageInvitationTemplate {
    pub invitation: Invitation,
    pub all_templates: Vec<InvitationTemplate>,
    pub is_dev: bool,
    pub user: Option<User>,
    pub guests: Vec<Guest>,
    pub groups: Vec<GuestGroup>,
    pub rsvps: Vec<Rsvp>,
    pub all_songs: Vec<Song>,
    // Pagination fields
    pub current_page: i32,
    pub total_pages: i32,
    pub total_items: i64,
    pub start_range: i64,
    pub end_range: i64,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub invitations: Vec<Invitation>,
    pub user: Option<User>,
    pub is_dev: bool,
    pub referral: Option<Referral>,
    pub total_commission: i32,
    pub referral_history: Vec<ReferralHistory>,
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

        let invitations = sqlx::query_as::<_, InvitationRow>("SELECT * FROM invitations WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC")
            .bind(uid)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                let bride: Person = from_value(r.bride_data.clone()).unwrap_or_default();
                let groom: Person = from_value(r.groom_data.clone()).unwrap_or_default();
                let bride_name_short = bride.name.clone();
                let groom_name_short = groom.name.clone();
                Invitation {
                    slug: r.slug,
                    template_name: r.template_name,
                    couple_name_short: r.couple_name_short,
                    bride_name_short,
                    groom_name_short,
                    bride,
                    groom,
                    event_date: format_date_for_display(&r.event_date),
                    ceremony: from_value(r.ceremony_data).unwrap_or_default(),
                    reception: from_value(r.reception_data).unwrap_or_default(),
                    quote: from_value(r.quote_data).unwrap_or_default(),
                    gallery_images: Vec::new(),
                    gallery_videos: Vec::new(),
                    gift_accounts: Vec::new(),
                    song_url: String::new(),
                    song_id: r.song_id,
                    plan_name: r.plan_name.unwrap_or_else(|| "NOBLE".to_string()),
                    ai_chat_enabled: r.ai_chat_enabled,
                    ai_usage_count: r.ai_usage_count,
                    ai_custom_knowledge: r.ai_custom_knowledge.unwrap_or_default(),
                    ai_language: r.ai_language.clone(),
                    recipient_name: "Guest & Partner".to_string(),
                    event_date_iso: "2026-05-24T08:00:00".to_string(),
                    reception_date_iso: "2026-05-24T08:00:00".to_string(),
                    rsvps: Vec::new(),
                    custom_song_url: r.custom_song_url.unwrap_or_default(),
                    background_video_url: r.background_video_url.unwrap_or_default(),
                    hero_video_position: r.hero_video_position.unwrap_or(50),
                    stories: from_value(r.stories_data.unwrap_or(json!([]))).unwrap_or_default(),
                    playlist: from_value(r.playlist.unwrap_or(json!([]))).unwrap_or_default(),
                    is_preview: false,
                }
            })
            .collect();

        // Auto-generate referral if not exists
        let mut referral = sqlx::query_as::<_, Referral>("SELECT * FROM referrals WHERE user_id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

        if referral.is_none() {
            let code = format!("CSTL-{}-{}", 
                user.as_ref().and_then(|u| u.name.clone()).unwrap_or_else(|| "MEMBER".to_string()).split_whitespace().next().unwrap_or("MEMBER").to_uppercase(),
                Uuid::new_v4().to_string().chars().take(4).collect::<String>().to_uppercase()
            );
            
            let inserted = sqlx::query_as::<_, Referral>(
                "INSERT INTO referrals (user_id, code, referrer_name, discount_percent, commission_percent)
                 VALUES ($1, $2, $3, $4, $5) RETURNING *"
            )
            .bind(uid)
            .bind(&code)
            .bind(user.as_ref().and_then(|u| u.name.clone()).unwrap_or_else(|| "Member".to_string()))
            .bind(10) // 10% discount default
            .bind(10) // 10% commission default
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);
            
            referral = inserted;
        }

        let mut total_commission = 0;
        let mut referral_history = Vec::new();

        if let Some(ref ref_data) = referral {
            let code = &ref_data.code;
            
            // Query for history using JOIN on bookings and users
            referral_history = sqlx::query_as::<_, ReferralHistory>(
                "SELECT 
                    COALESCE(u.name, 'Guest') as user_name, 
                    b.target_plan, 
                    b.amount, 
                    (b.amount * r.commission_percent / 100) as commission_earned, 
                    b.created_at
                 FROM bookings b
                 JOIN referrals r ON b.voucher_code = r.code
                 LEFT JOIN users u ON b.user_id = u.id
                 WHERE b.voucher_code = $1 AND b.status = 'SUCCESS'
                 ORDER BY b.created_at DESC"
            )
            .bind(code)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            // Calculate total commission
            total_commission = referral_history.iter().map(|h| h.commission_earned).sum();
        }

        HtmlTemplate(DashboardTemplate {
            invitations,
            user,
            is_dev: state.is_dev,
            referral,
            total_commission,
            referral_history,
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
                    "SELECT * FROM invitations WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC"
                )
                .bind(uid)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
            }
        }
    }

    let templates: Vec<InvitationTemplate> = sqlx::query_as::<_, InvitationTemplate>(
        "SELECT * FROM templates WHERE status = 'PUBLISHED' AND is_featured = TRUE ORDER BY id ASC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(sanitize_template_urls)
    .collect();

    HtmlTemplate(HomeTemplate { user, invitations, templates, is_dev: state.is_dev }).into_response()
}

pub async fn invitation_detail(
    Path(slug): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let param_to = params.get("to").cloned().unwrap_or_else(|| "none".to_string());
    let param_preview = params.get("preview_theme").cloned().unwrap_or_else(|| "none".to_string());
    let cache_field = format!("{}_{}", param_to, param_preview);
    
    // Check Redis Cache
    if let Ok(mut conn) = state.redis.get().await {
        let cache_key = format!("invitation_cache:{}", slug);
        if let Ok(cached) = redis::cmd("HGET").arg(&cache_key).arg(&cache_field).query_async::<String>(&mut conn).await {
            if let Ok(invitation) = serde_json::from_str::<Invitation>(&cached) {
                let template_name = invitation.template_name.clone();
                return render_invitation_template(&template_name, invitation, state.is_dev).into_response();
            }
        }
    }

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
        "SELECT * FROM invitations WHERE slug = $1 AND deleted_at IS NULL"
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
                "SELECT url, photo_type FROM invitation_photos WHERE invitation_id = $1 ORDER BY \"order\" ASC"
            )
            .bind(row.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
            
            let gallery_images: Vec<String> = photos.iter()
                .filter(|p| p.get::<&str, _>("photo_type") == "gallery")
                .map(|p| p.get::<String, _>("url"))
                .collect();

            let gallery_videos: Vec<String> = photos.into_iter()
                .filter(|p| p.get::<&str, _>("photo_type") == "gallery_video")
                .map(|p| p.get::<String, _>("url"))
                .collect();

            let mut template_name = row.template_name.clone();
            let mut ai_language = row.ai_language.clone();
            let mut final_song_id = row.song_id;
            
            let mut recipient_name = "Guest & Partner".to_string();
            
            // Override with preview_theme if provided
            if let Some(preview) = params.get("preview_theme") {
                template_name = preview.clone();
            } else if let Some(gs) = params.get("to") {
                recipient_name = gs.clone(); // Default to the query param value
                let guest = sqlx::query_as::<_, Guest>("SELECT id, invitation_id, name, category, template_override, slug, is_sent, COALESCE(ai_language, '') as ai_language, song_id, created_at FROM guests WHERE invitation_id = $1 AND (slug = $2 OR name = $2)")
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

                    if let Some(sid) = g.song_id {
                        final_song_id = Some(sid);
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
                        let group = sqlx::query_as::<_, GuestGroup>("SELECT id, invitation_id, name, template_name, COALESCE(ai_language, '') as ai_language, song_id, created_at FROM invitation_groups WHERE invitation_id = $1 AND name = $2")
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

                            // If guest song is empty, use group song
                            if g.song_id.is_none() && grp.song_id.is_some() {
                                final_song_id = grp.song_id;
                            }
                        }
                    }
                }
            }

            let event_date_iso = parse_event_date_to_iso(&row.event_date);

            let mut ceremony: EventDetails = from_value(row.ceremony_data).unwrap_or_default();
            let mut reception: EventDetails = from_value(row.reception_data).unwrap_or_default();
            let reception_date_iso = parse_event_date_to_iso(&reception.date);
            
            // Format dates for display
            let event_date = format_date_for_display(&row.event_date);
            if ceremony.date.is_empty() || ceremony.date.contains('-') {
                ceremony.date = event_date.clone();
            } else {
                ceremony.date = format_date_for_display(&ceremony.date);
            }
            reception.date = format_date_for_display(&reception.date);

            let bride: Person = from_value(row.bride_data).unwrap_or_default();
            let groom: Person = from_value(row.groom_data).unwrap_or_default();
            let bride_name_short = bride.name.clone();
            let groom_name_short = groom.name.clone();

            let mut final_song_url = row.custom_song_url.clone().unwrap_or_else(|| song_url.clone());
            if let Some(sid) = final_song_id {
                // If there's a specific song_id (from guest/group), it overrides the invitation's custom song
                let song = sqlx::query_as::<_, Song>("SELECT * FROM songs WHERE id = $1")
                    .bind(sid)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or_default();
                if let Some(s) = song {
                    final_song_url = s.file_path;
                }
            }

            let invitation = Invitation {
                slug: row.slug.clone(),
                template_name: template_name.clone(),
                couple_name_short: row.couple_name_short,
                bride_name_short,
                groom_name_short,
                bride,
                groom,
                event_date,
                ceremony,
                reception,
                quote: from_value(row.quote_data).unwrap_or_default(),
                gallery_images,
                gallery_videos,
                gift_accounts,
                song_url: final_song_url.clone(),
                song_id: final_song_id,
                custom_song_url: row.custom_song_url.unwrap_or_default(),
                background_video_url: row.background_video_url.unwrap_or_default(),
                hero_video_position: row.hero_video_position.unwrap_or(50),
                stories: from_value(row.stories_data.unwrap_or(json!([]))).unwrap_or_default(),
                playlist: {
                    let mut list: Vec<String> = from_value(row.playlist.unwrap_or(json!([]))).unwrap_or_default();
                    if list.is_empty() && !final_song_url.is_empty() {
                        list.push(final_song_url.clone());
                    }
                    list
                },
                plan_name: row.plan_name.unwrap_or_else(|| "NOBLE".to_string()),
                ai_chat_enabled: row.ai_chat_enabled,
                ai_usage_count: row.ai_usage_count,
                ai_custom_knowledge: row.ai_custom_knowledge.unwrap_or_default(),
                ai_language: ai_language,
                recipient_name: recipient_name,
                event_date_iso: event_date_iso,
                reception_date_iso: reception_date_iso,
                rsvps: sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE invitation_id = $1 ORDER BY created_at DESC")
                    .bind(row.id)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default(),
                is_preview: params.contains_key("preview_theme"),
            };

            // Set to Redis Cache
            if let Ok(mut conn) = state.redis.get().await {
                let cache_key = format!("invitation_cache:{}", slug);
                let json_data = invitation.to_json_context();
                let _ = redis::cmd("HSET").arg(&cache_key).arg(&cache_field).arg(&json_data).query_async::<()>(&mut conn).await;
                let _ = redis::cmd("EXPIRE").arg(&cache_key).arg(3600).query_async::<()>(&mut conn).await; // 1 hour TTL
            }

            render_invitation_template(template_name.as_str(), invitation, state.is_dev)
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
                    bride_name_short: "Nazma".to_string(),
                    groom_name_short: "Guntur".to_string(),
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
                        enabled: true,
                        date: "Sabtu, 12 Desember 2026".to_string(),
                        time: "09:00 - 10:00 WIB".to_string(),
                        venue: "Masjid Raya".to_string(),
                        address: "Jl. Diponegoro No.1, Jakarta".to_string(),
                        maps_url: "https://maps.app.goo.gl/xxx".to_string(),
                    },
                    reception: EventDetails {
                        enabled: true,
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
                    gallery_videos: vec![
                        "https://assets.mixkit.co/videos/preview/mixkit-wedding-couple-walking-in-the-field-1234-large.mp4".to_string(),
                        "https://assets.mixkit.co/videos/preview/mixkit-wedding-couple-walking-in-a-forest-40439-large.mp4".to_string(),
                    ],
                    gift_accounts: vec![
                        GiftAccount {
                            bank_name: "BCA".to_string(),
                            account_number: "1234567890".to_string(),
                            account_holder: "Nazma Putri".to_string(),
                        },
                    ],
                    song_url: song_url.clone(),
                    song_id: None,
                    plan_name: "NOBLE".to_string(),
                    ai_chat_enabled: false,
                    ai_usage_count: 0,
                    ai_custom_knowledge: String::new(),
                    ai_language: "id".to_string(),
                    recipient_name: "Guest & Partner".to_string(),
                    event_date_iso: "2026-12-12T08:00:00".to_string(),
                    reception_date_iso: "2026-12-12T08:00:00".to_string(),
                    rsvps: Vec::new(),
                    custom_song_url: String::new(),
                    background_video_url: "https://assets.mixkit.co/videos/preview/mixkit-wedding-couple-walking-in-the-field-1234-large.mp4".to_string(),
                    hero_video_position: 50,
                    stories: vec![
                        Story {
                            id: "1".to_string(),
                            title: "Awal Pertemuan".to_string(),
                            date: "Juni 2025".to_string(),
                            description: "Di hari pertama bertemu, tawa tercipta di antara lemparan bowling, cerita mengalir di sela sushi, dan es krim menjadi saksi awal kisah kami.".to_string(),
                            image_url: "https://images.unsplash.com/photo-1522071820081-009f0129c71c?w=600&q=80".to_string(),
                        },
                        Story {
                            id: "2".to_string(),
                            title: "Lamaran".to_string(),
                            date: "November 2025".to_string(),
                            description: "Seiring waktu, rasa itu tumbuh menjadi keyakinan. Di hadapan keluarga, kami mengikat janji dalam sebuah langkah awal menuju masa depan bersama.".to_string(),
                            image_url: "https://images.unsplash.com/photo-1515934751635-c81c6bc9a2d8?w=600&q=80".to_string(),
                        },
                        Story {
                            id: "3".to_string(),
                            title: "Pernikahan".to_string(),
                            date: "Desember 2026".to_string(),
                            description: "Kini, kami memilih untuk melangkah lebih jauh. Dalam ikatan suci pernikahan, kami berjanji untuk saling menjaga, mencintai, dan tumbuh bersama.".to_string(),
                            image_url: "https://images.unsplash.com/photo-1519741497674-611481863552?w=600&q=80".to_string(),
                        },
                    ],
                    playlist: vec![],
                    is_preview: true,
                };
                
                render_invitation_template(template_name, invitation, state.is_dev)
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
    Query(_params): Query<HashMap<String, String>>,
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
        "SELECT * FROM invitations WHERE slug = $1 AND user_id = $2 AND deleted_at IS NULL"
    )
    .bind(&slug)
    .bind(user_id.unwrap())
    .fetch_optional(&state.db)
    .await
    .unwrap();

    match row {
        Some(row) => {
            let event_date_iso = parse_event_date_to_iso(&row.event_date);
            let reception_temp: EventDetails = from_value(row.reception_data.clone()).unwrap_or_default();
            let reception_date_iso = parse_event_date_to_iso(&reception_temp.date);
            let bride: Person = from_value(row.bride_data).unwrap_or_default();
            let groom: Person = from_value(row.groom_data).unwrap_or_default();
            let bride_name_short = bride.name.clone();
            let groom_name_short = groom.name.clone();

            let gallery_images = sqlx::query_scalar::<_, String>(
                "SELECT url FROM invitation_photos WHERE invitation_id = $1 AND photo_type = 'gallery' ORDER BY \"order\" ASC"
            )
            .bind(row.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let gallery_videos = sqlx::query_scalar::<_, String>(
                "SELECT url FROM invitation_photos WHERE invitation_id = $1 AND photo_type = 'gallery_video' ORDER BY \"order\" ASC"
            )
            .bind(row.id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let invitation = Invitation {
                slug: row.slug,
                template_name: row.template_name,
                couple_name_short: row.couple_name_short,
                bride_name_short,
                groom_name_short,
                bride,
                groom,
                event_date: format_date_for_display(&row.event_date),
                ceremony: from_value(row.ceremony_data).unwrap_or_default(),
                reception: from_value(row.reception_data).unwrap_or_default(),
                quote: from_value(row.quote_data).unwrap_or_default(),
                gallery_images,
                gallery_videos,
                gift_accounts: sqlx::query_as::<_, GiftAccount>("SELECT bank_name, account_number, account_holder FROM gift_accounts WHERE invitation_id = $1").bind(row.id).fetch_all(&state.db).await.unwrap_or_default(),
                song_url: String::new(),
                song_id: row.song_id,
                plan_name: row.plan_name.unwrap_or_else(|| "NOBLE".to_string()),
                ai_chat_enabled: row.ai_chat_enabled,
                ai_usage_count: row.ai_usage_count,
                ai_custom_knowledge: row.ai_custom_knowledge.unwrap_or_default(),
                ai_language: row.ai_language.clone(),
                recipient_name: "Guest & Partner".to_string(),
                event_date_iso,
                reception_date_iso,
                rsvps: sqlx::query_as::<_, Rsvp>("SELECT * FROM rsvps WHERE invitation_id = $1 ORDER BY created_at DESC")
                    .bind(row.id)
                    .fetch_all(&state.db)
                    .await
                    .unwrap_or_default(),
                custom_song_url: row.custom_song_url.unwrap_or_default(),
                background_video_url: row.background_video_url.unwrap_or_default(),
                hero_video_position: row.hero_video_position.unwrap_or(50),
                stories: from_value(row.stories_data.unwrap_or(json!([]))).unwrap_or_default(),
                playlist: from_value(row.playlist.unwrap_or(json!([]))).unwrap_or_default(),
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

            let all_templates = get_all_templates(&state.db, true).await;
            let all_songs = sqlx::query_as::<_, Song>("SELECT * FROM songs WHERE is_active = true ORDER BY title ASC")
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

            // Pagination parameters (delegated to client-side datatable, preserved for template compatibility)
            let current_page: i32 = 1;
            let guests = sqlx::query_as::<_, Guest>("SELECT id, invitation_id, name, category, template_override, slug, is_sent, COALESCE(ai_language, '') as ai_language, song_id, created_at FROM guests WHERE invitation_id = $1 ORDER BY created_at DESC")
                .bind(row.id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
            let total_items: i64 = guests.len() as i64;
            let total_pages: i32 = 1;
            let start_range: i64 = if total_items == 0 { 0 } else { 1 };
            let end_range: i64 = total_items;
            
            let groups = sqlx::query_as::<_, GuestGroup>("SELECT id, invitation_id, name, template_name, COALESCE(ai_language, '') as ai_language, song_id, created_at FROM invitation_groups WHERE invitation_id = $1 ORDER BY name ASC")
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
                all_songs,
                current_page,
                total_pages,
                total_items,
                start_range,
                end_range,
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
        "SELECT * FROM invitations WHERE slug = $1 AND user_id = $2 AND deleted_at IS NULL"
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
    let mut existing_gallery = Vec::new();
    let mut bank_names = Vec::new();
    let mut account_numbers = Vec::new();
    let mut account_holders = Vec::new();
    let mut custom_song_url = row.custom_song_url.clone();
    let mut background_video_url = row.background_video_url.clone();
    let mut gallery_video_paths = Vec::new();
    let mut existing_gallery_videos = Vec::new();
    
    let mut story_titles = Vec::new();
    let mut story_dates = Vec::new();
    let mut story_descriptions = Vec::new();
    let mut story_image_urls = Vec::new();
    let mut story_image_files = Vec::new();
    let mut playlist_paths = Vec::new();
    let mut existing_playlist = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        tracing::info!("Received multipart field: {}", name);
        
        if name == "gallery[]" {
            let filename = Uuid::new_v4().to_string() + ".webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            tracing::info!("Gallery photo size: {} bytes", data.len());
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1600).await {
                    gallery_paths.push(format!("/uploads/{}", filename));
                }
            }
        } else if name.ends_with("_photo") {
            let filename = Uuid::new_v4().to_string() + ".webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            tracing::info!("Couple photo size ({}): {} bytes", name, data.len());
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1600).await {
                    photo_paths.insert(name, format!("/uploads/{}", filename));
                }
            }
        } else if name == "bank_name[]" {
            bank_names.push(field.text().await.unwrap_or_default());
        } else if name == "account_number[]" {
            account_numbers.push(field.text().await.unwrap_or_default());
        } else if name == "account_holder[]" {
            account_holders.push(field.text().await.unwrap_or_default());
        } else if name == "existing_gallery[]" {
            existing_gallery.push(field.text().await.unwrap_or_default());
        } else if name == "custom_song" {
            let filename = Uuid::new_v4().to_string() + ".mp3";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() && data.len() < 10 * 1024 * 1024 { // 10MB limit
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "audio/mpeg").await {
                    custom_song_url = Some(format!("/uploads/{}", filename));
                }
            }
        } else if name == "background_video" {
            let filename = Uuid::new_v4().to_string() + ".mp4";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() && data.len() < 25 * 1024 * 1024 { // 25MB limit
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "video/mp4").await {
                    background_video_url = Some(format!("/uploads/{}", filename));
                }
            }
        } else if name == "gallery_video[]" {
            let plan = row.plan_name.as_deref().unwrap_or("NOBLE");
            let (_, max_size) = match plan {
                "DYNASTY" => (10, 25 * 1024 * 1024),
                "ROYAL" => (5, 20 * 1024 * 1024),
                _ => (3, 10 * 1024 * 1024),
            };

            let filename = Uuid::new_v4().to_string() + ".mp4";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            
            if !data.is_empty() && data.len() <= max_size {
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "video/mp4").await {
                    gallery_video_paths.push(format!("/uploads/{}", filename));
                }
            }
        } else if name == "existing_gallery_video[]" {
            existing_gallery_videos.push(field.text().await.unwrap_or_default());
        } else if name == "story_title[]" {
            story_titles.push(field.text().await.unwrap_or_default());
        } else if name == "story_date[]" {
            story_dates.push(field.text().await.unwrap_or_default());
        } else if name == "story_description[]" {
            story_descriptions.push(field.text().await.unwrap_or_default());
        } else if name == "story_image_url[]" {
            story_image_urls.push(field.text().await.unwrap_or_default());
        } else if name == "story_image_file[]" {
            let filename = Uuid::new_v4().to_string() + ".webp";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() {
                if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1200).await {
                    story_image_files.push(Some(format!("/uploads/{}", filename)));
                } else {
                    story_image_files.push(None);
                }
            } else {
                story_image_files.push(None);
            }
        } else if name == "playlist[]" {
            let filename = Uuid::new_v4().to_string() + ".mp3";
            let path = format!("static/uploads/{}", filename);
            let data = field.bytes().await.unwrap_or_default();
            if !data.is_empty() && data.len() < 10 * 1024 * 1024 {
                if process_and_save_file(&state.s3_client, &state.s3_bucket, data, path.clone(), "audio/mpeg").await {
                    playlist_paths.push(format!("/uploads/{}", filename));
                }
            }
        } else if name == "existing_playlist[]" {
            existing_playlist.push(field.text().await.unwrap_or_default());
        } else {
            let value = field.text().await.unwrap();
            fields.insert(name, value);
        }
    }

    // Update JSON Data
    let mut bride: Person = from_value(row.bride_data.clone()).unwrap_or_default();
    if let Some(val) = fields.get("bride_name") { bride.name = val.clone(); }
    if let Some(val) = fields.get("bride_full_name") { bride.full_name = val.clone(); }
    if let Some(val) = fields.get("bride_father") { bride.father_name = val.clone(); }
    if let Some(val) = fields.get("bride_mother") { bride.mother_name = val.clone(); }
    if let Some(val) = photo_paths.get("bride_photo") { bride.image_url = val.clone(); }

    let mut groom: Person = from_value(row.groom_data.clone()).unwrap_or_default();
    if let Some(val) = fields.get("groom_name") { groom.name = val.clone(); }
    if let Some(val) = fields.get("groom_full_name") { groom.full_name = val.clone(); }
    if let Some(val) = fields.get("groom_father") { groom.father_name = val.clone(); }
    if let Some(val) = fields.get("groom_mother") { groom.mother_name = val.clone(); }
    if let Some(val) = photo_paths.get("groom_photo") { groom.image_url = val.clone(); }

    let mut ceremony: EventDetails = from_value(row.ceremony_data.clone()).unwrap_or_default();
    if fields.contains_key("ceremony_enabled") {
        ceremony.enabled = fields.get("ceremony_enabled").map(|v| v == "on").unwrap_or(false);
    } else {
        ceremony.enabled = !ceremony.venue.is_empty() || !ceremony.address.is_empty() || ceremony.enabled;
    }
    if let Some(val) = fields.get("ceremony_time") { ceremony.time = val.clone(); }
    if let Some(val) = fields.get("ceremony_venue") { ceremony.venue = val.clone(); }
    if let Some(val) = fields.get("ceremony_address") { ceremony.address = val.clone(); }
    if let Some(val) = fields.get("ceremony_maps") { ceremony.maps_url = val.clone(); }

    let mut reception: EventDetails = from_value(row.reception_data.clone()).unwrap_or_default();
    if fields.contains_key("reception_enabled") {
        reception.enabled = fields.get("reception_enabled").map(|v| v == "on").unwrap_or(false);
    } else {
        reception.enabled = !reception.venue.is_empty() || !reception.address.is_empty() || reception.enabled;
    }
    if let Some(val) = fields.get("reception_date") { reception.date = val.clone(); }
    if let Some(val) = fields.get("reception_time") { reception.time = val.clone(); }
    if let Some(val) = fields.get("reception_venue") { reception.venue = val.clone(); }
    if let Some(val) = fields.get("reception_address") { reception.address = val.clone(); }
    if let Some(val) = fields.get("reception_maps") { reception.maps_url = val.clone(); }

    let mut quote: Quote = from_value(row.quote_data.clone()).unwrap_or_default();
    if let Some(val) = fields.get("quote_text") {
        quote.text = val.trim().to_string();
    }
    if let Some(val) = fields.get("quote_source") {
        quote.source = val.trim().to_string();
    }

    let couple_name_short = fields.get("couple_name_short")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(row.couple_name_short);

    let event_date_raw = fields.get("event_date")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| row.event_date.clone());

    tracing::info!("--- UPDATE INVITATION DEBUG ---");
    tracing::info!("fields keys: {:?}", fields.keys().collect::<Vec<&String>>());
    tracing::info!("fields event_date: {:?}", fields.get("event_date"));
    tracing::info!("row.event_date: {:?}", row.event_date);
    tracing::info!("event_date_raw: {:?}", event_date_raw);
    
    // Keep raw date format like YYYY-MM-DD in the database
    let event_date = event_date_raw;
    tracing::info!("raw event_date: {:?}", event_date);
    
    // Sync ceremony date with the main event date
    ceremony.date = event_date.clone();
    let ai_chat_enabled = fields.get("ai_chat_enabled").map(|v| v == "on").unwrap_or(false);
    let ai_custom_knowledge = fields.get("ai_custom_knowledge")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| row.ai_custom_knowledge.clone().unwrap_or_default());
    let template_name = fields.get("template_name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(row.template_name);
    let final_ai_language = fields.get("ai_language")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(row.ai_language.clone());
    let hero_video_position = fields.get("hero_video_position")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(row.hero_video_position.unwrap_or(50));
    let song_id = fields.get("song_id")
        .and_then(|s| if s.is_empty() { None } else { Uuid::parse_str(s).ok() })
        .or(row.song_id);

    let mut final_slug = row.slug.clone();
    if let Some(new_slug) = fields.get("slug") {
        let new_slug = new_slug.trim().to_lowercase();
        if !new_slug.is_empty() && new_slug != row.slug {
            // Validate slug format
            if new_slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
                // Check uniqueness
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invitations WHERE slug = $1 AND id != $2")
                    .bind(&new_slug)
                    .bind(row.id)
                    .fetch_one(&state.db)
                    .await
                    .unwrap_or(0);
                
                if count == 0 {
                    final_slug = new_slug;
                }
            }
        }
    }
    
    // Process Stories
    let mut final_stories = Vec::new();
    let story_count = story_titles.len();
    for i in 0..story_count {
        let title = story_titles.get(i).cloned().unwrap_or_default();
        let date = story_dates.get(i).cloned().unwrap_or_default();
        let description = story_descriptions.get(i).cloned().unwrap_or_default();
        
        // Image logic: use new file if provided, otherwise fallback to existing url
        let image_url = if let Some(Some(new_path)) = story_image_files.get(i) {
            new_path.clone()
        } else {
            story_image_urls.get(i).cloned().unwrap_or_default()
        };

        if !title.is_empty() {
            final_stories.push(Story {
                id: Uuid::new_v4().to_string(),
                title,
                date,
                description,
                image_url,
            });
        }
    }
    
    // S3 Cleanup for stories
    let old_stories: Vec<Story> = from_value(row.stories_data.clone().unwrap_or(serde_json::json!([]))).unwrap_or_default();
    let new_story_image_urls: Vec<String> = final_stories.iter().map(|s| s.image_url.clone()).collect();
    for os in old_stories {
        if !os.image_url.is_empty() && !new_story_image_urls.contains(&os.image_url) {
            delete_s3_file(&state.s3_client, &state.s3_bucket, &os.image_url).await;
        }
    }

    // Pre-Cleanup: Handle Song and Background Video
    if custom_song_url != row.custom_song_url {
        if let Some(old_url) = &row.custom_song_url {
            delete_s3_file(&state.s3_client, &state.s3_bucket, old_url).await;
        }
    }
    if background_video_url != row.background_video_url {
        if let Some(old_url) = &row.background_video_url {
            delete_s3_file(&state.s3_client, &state.s3_bucket, old_url).await;
        }
    }

    // Pre-Cleanup: Handle Couple Photos
    let old_bride: Person = from_value(row.bride_data.clone()).unwrap_or_default();
    if let Some(new_url) = photo_paths.get("bride_photo") {
        if &old_bride.image_url != new_url {
            delete_s3_file(&state.s3_client, &state.s3_bucket, &old_bride.image_url).await;
        }
    }
    let old_groom: Person = from_value(row.groom_data.clone()).unwrap_or_default();
    if let Some(new_url) = photo_paths.get("groom_photo") {
        if &old_groom.image_url != new_url {
            delete_s3_file(&state.s3_client, &state.s3_bucket, &old_groom.image_url).await;
        }
    }

    // Handle Playlist (Plan-based limits)
    let plan = row.plan_name.as_deref().unwrap_or("NOBLE");
    let song_limit = match plan {
        "DYNASTY" => 5,
        "ROYAL" => 3,
        _ => 1,
    };
    
    let mut final_playlist = existing_playlist;
    final_playlist.extend(playlist_paths);
    
    // If playlist is empty but there's a custom song, use it as fallback
    if final_playlist.is_empty() {
        if let Some(url) = &custom_song_url {
            if !url.is_empty() {
                final_playlist.push(url.clone());
            }
        }
    }
    final_playlist.truncate(song_limit);

    sqlx::query(
        "UPDATE invitations SET couple_name_short = $1, event_date = $2, bride_data = $3, groom_data = $4, ceremony_data = $5, reception_data = $6, quote_data = $7, ai_chat_enabled = $8, ai_custom_knowledge = $9, ai_language = $10, template_name = $11, song_id = $12, custom_song_url = $13, background_video_url = $14, stories_data = $15, hero_video_position = $16, playlist = $17, slug = $18 WHERE id = $19"
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
    .bind(song_id)
    .bind(custom_song_url)
    .bind(background_video_url)
    .bind(json!(final_stories))
    .bind(hero_video_position)
    .bind(json!(final_playlist))
    .bind(&final_slug)
    .bind(row.id)
    .execute(&state.db)
    .await
    .unwrap();

    tracing::info!("Successfully updated invitation record for id: {}", row.id);

    // Handle Gallery Management: Delete all existing gallery photos and re-insert (kept + new)
    let old_gallery_items = sqlx::query("SELECT url, photo_type FROM invitation_photos WHERE invitation_id = $1")
        .bind(row.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let _ = sqlx::query("DELETE FROM invitation_photos WHERE invitation_id = $1 AND photo_type = 'gallery'")
        .bind(row.id)
        .execute(&state.db)
        .await;

    // S3 Cleanup for removed gallery items
    for p in old_gallery_items {
        let url: String = p.get("url");
        let ptype: String = p.get("photo_type");
        if ptype == "gallery" && !existing_gallery.contains(&url) {
            delete_s3_file(&state.s3_client, &state.s3_bucket, &url).await;
        } else if ptype == "gallery_video" && !existing_gallery_videos.contains(&url) {
            delete_s3_file(&state.s3_client, &state.s3_bucket, &url).await;
        }
    }

    let mut final_gallery = existing_gallery;
    final_gallery.extend(gallery_paths);

    for (i, path) in final_gallery.into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO invitation_photos (invitation_id, url, photo_type, \"order\") VALUES ($1, $2, $3, $4)"
        )
        .bind(row.id)
        .bind(path)
        .bind("gallery")
        .bind(i as i32)
        .execute(&state.db)
        .await
        .unwrap();
    }

    // Handle Gallery Video Management
    let _ = sqlx::query("DELETE FROM invitation_photos WHERE invitation_id = $1 AND photo_type = 'gallery_video'")
        .bind(row.id)
        .execute(&state.db)
        .await;

    let mut final_gallery_videos = existing_gallery_videos;
    final_gallery_videos.extend(gallery_video_paths);

    let plan = row.plan_name.as_deref().unwrap_or("NOBLE");
    let max_count = match plan {
        "DYNASTY" => 10,
        "ROYAL" => 5,
        _ => 3,
    };

    for (i, path) in final_gallery_videos.into_iter().take(max_count).enumerate() {
        sqlx::query(
            "INSERT INTO invitation_photos (invitation_id, url, photo_type, \"order\") VALUES ($1, $2, $3, $4)"
        )
        .bind(row.id)
        .bind(path)
        .bind("gallery_video")
        .bind(i as i32)
        .execute(&state.db)
        .await
        .unwrap();
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

    Redirect::to(&format!("/invitation/{}/manage", final_slug)).into_response()
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
    Form(payload): Form<RsvpForm>,
) -> impl IntoResponse {
    tracing::info!("RSVP received: {:?}", payload);

    // Attempt to insert RSVP into the database
    let result = sqlx::query(
        "INSERT INTO rsvps (invitation_id, name, attendance, guests, message) \
         SELECT id, $1, $2, $3, $4 FROM invitations WHERE slug = $5",
    )
    .bind(&payload.name)
    .bind(&payload.attendance)
    .bind(payload.guests as i32)
    .bind(&payload.message)
    .bind(&payload.invitation_slug)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            invalidate_invitation_cache(&state, &payload.invitation_slug).await;
            Json(serde_json::json!({"status": "ok"})).into_response()
        },
        Err(e) => {
            tracing::error!("Failed to insert RSVP: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "error",
                    "error": e.to_string()
                }))
            ).into_response()
        }
    }
}pub async fn sitemap(State(state): State<AppState>) -> impl IntoResponse {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
    <url>
        <loc>https://castellant.com/</loc>
        <lastmod>{}</lastmod>
        <changefreq>daily</changefreq>
        <priority>1.0</priority>
    </url>
    <url>
        <loc>https://castellant.com/templates</loc>
        <lastmod>{}</lastmod>
        <changefreq>weekly</changefreq>
        <priority>0.9</priority>
    </url>"#, today, today);

    // Dynamic: fetch public invitation slugs with created_at as lastmod
    // Only include: not deleted, has a valid slug, and has paid plan (plan_name IS NOT NULL)
    let rows = sqlx::query(
        "SELECT slug, TO_CHAR(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') as lastmod \
         FROM invitations \
         WHERE deleted_at IS NULL \
           AND slug != '' \
           AND slug NOT LIKE 'deleted-%' \
           AND plan_name IS NOT NULL \
         ORDER BY created_at DESC \
         LIMIT 500"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in rows {
        let slug: String = row.get("slug");
        if slug.is_empty() || slug.starts_with("deleted-") {
            continue;
        }
        let lastmod: String = row.try_get("lastmod").unwrap_or_else(|_| today.clone());
        xml.push_str(&format!(r#"
    <url>
        <loc>https://castellant.com/invitation/{}</loc>
        <lastmod>{}</lastmod>
        <changefreq>monthly</changefreq>
        <priority>0.6</priority>
    </url>"#, slug, lastmod));
    }

    // Add SEO Landing Pages
    let seo_pages = [
        "/undangan-digital",
        "/undangan-pernikahan",
        "/undangan-pernikahan-ai",
        "/undangan-pernikahan-banyak-template-satu-undangan",
        "/blog"
    ];
    for page in seo_pages.iter() {
        xml.push_str(&format!(r#"
    <url>
        <loc>https://castellant.com{}</loc>
        <lastmod>{}</lastmod>
        <changefreq>weekly</changefreq>
        <priority>0.9</priority>
    </url>"#, page, today));
    }

    // Dynamic: fetch blog posts
    let blog_rows = sqlx::query(
        "SELECT slug, TO_CHAR(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD') as lastmod \
         FROM blog_posts \
         WHERE is_published = true \
         ORDER BY published_at DESC \
         LIMIT 100"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for row in blog_rows {
        let slug: String = row.get("slug");
        let lastmod: String = row.try_get("lastmod").unwrap_or_else(|_| today.clone());
        xml.push_str(&format!(r#"
    <url>
        <loc>https://castellant.com/blog/{}</loc>
        <lastmod>{}</lastmod>
        <changefreq>monthly</changefreq>
        <priority>0.8</priority>
    </url>"#, slug, lastmod));
    }

    xml.push_str("\n</urlset>");

    Response::builder()
        .header("Content-Type", "application/xml")
        .header("Cache-Control", "public, max-age=3600")
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
        bride_name_short: payload.bride_name.clone(),
        groom_name_short: payload.groom_name.clone(),
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
        event_date: format_date_for_display(&payload.ceremony_date),
        ceremony: EventDetails {
            enabled: true,
            date: format_date_for_display(&payload.ceremony_date),
            time: payload.ceremony_time,
            venue: payload.ceremony_venue,
            address: payload.ceremony_address,
            maps_url: payload.ceremony_maps,
        },
        reception: EventDetails {
            enabled: true,
            date: format_date_for_display(&payload.reception_date),
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
        gallery_videos: Vec::new(),
        gift_accounts: vec![
            GiftAccount {
                bank_name: "BCA".to_string(),
                account_number: "1234567890".to_string(),
                account_holder: "Preview User".to_string(),
            },
        ],
        song_url: "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3".to_string(),
        song_id: None,
        is_preview: true,
        plan_name: "NOBLE".to_string(),
        ai_chat_enabled: false,
        ai_usage_count: 0,
        ai_custom_knowledge: String::new(),
        ai_language: "id".to_string(),
        recipient_name: "Guest & Partner".to_string(),
        event_date_iso: "2026-05-24T08:00:00".to_string(),
        reception_date_iso: "2026-05-24T08:00:00".to_string(),
        rsvps: Vec::new(),
        custom_song_url: String::new(),
        background_video_url: String::new(),
        hero_video_position: 50,
        stories: Vec::new(),
        playlist: Vec::new(),
    };

    match payload.template_name.as_str() {
        "keraton-dark-invitation" => HtmlTemplate(KeratonDarkInvitationTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "loveanthem" => HtmlTemplate(LoveAnthemTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "cinemarry" => HtmlTemplate(CineMarryTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "royal-heritage" => HtmlTemplate(RoyalHeritageTemplate { invitation, is_dev: state.is_dev }).into_response(),
        "high-fashion-editorial" => HtmlTemplate(HighFashionEditorialTemplate { invitation: invitation.clone(), is_dev: state.is_dev }).into_response(),
        "reel-wedding" => HtmlTemplate(ReelWeddingTemplate { invitation, is_dev: state.is_dev }).into_response(),
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
        "trendvibe" => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: state.is_dev }).into_response(),
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
        "SELECT * FROM invitations WHERE slug = $1 AND deleted_at IS NULL"
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
            "SELECT id, invitation_id, name, category, template_override, slug, is_sent, COALESCE(ai_language, '') as ai_language, song_id, created_at FROM guests WHERE invitation_id = $1 AND slug = $2"
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
    pub song_id: Option<Uuid>,
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

    let (invitation_id, plan_name): (Uuid, Option<String>) = sqlx::query_as("SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2 AND deleted_at IS NULL")
        .bind(&slug)
        .bind(user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let guest_slug = payload.name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .replace(" ", "-");
    
    let ai_language = if plan_name.as_deref().unwrap_or("NOBLE") == "DYNASTY" {
        payload.ai_language.unwrap_or_default()
    } else {
        "".to_string()
    };

    sqlx::query(
        "INSERT INTO guests (invitation_id, name, category, slug, template_override, ai_language, song_id) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(invitation_id)
    .bind(&payload.name)
    .bind(&payload.category)
    .bind(&guest_slug)
    .bind(&payload.template_override)
    .bind(&ai_language)
    .bind(&payload.song_id)
    .execute(&state.db)
    .await
    .unwrap();

    invalidate_invitation_cache(&state, &slug).await;
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

    let (invitation_id, plan_name): (Uuid, Option<String>) = sqlx::query_as("SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2 AND deleted_at IS NULL")
        .bind(&slug)
        .bind(user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let ai_language = if plan_name.as_deref().unwrap_or("NOBLE") == "DYNASTY" {
        payload.ai_language.unwrap_or_default()
    } else {
        "".to_string()
    };

    sqlx::query(
        "UPDATE guests SET name = $1, category = $2, template_override = $3, ai_language = $4, song_id = $5 WHERE id = $6 AND invitation_id = $7"
    )
    .bind(&payload.name)
    .bind(&payload.category)
    .bind(&payload.template_override)
    .bind(&ai_language)
    .bind(&payload.song_id)
    .bind(guest_id)
    .bind(invitation_id)
    .execute(&state.db)
    .await
    .unwrap();

    invalidate_invitation_cache(&state, &slug).await;
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

    invalidate_invitation_cache(&state, &slug).await;
    Redirect::to(&format!("/invitation/{}/manage#guests", slug)).into_response()
}

pub async fn delete_guest(
    Path((slug, guest_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM guests WHERE id = $1").bind(guest_id).execute(&state.db).await.unwrap();
    invalidate_invitation_cache(&state, &slug).await;
    Redirect::to(&format!("/invitation/{}/manage#guests", slug)).into_response()
}

pub async fn toggle_guest_sent(
    Path((_slug, guest_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let row = sqlx::query!("SELECT is_sent FROM guests WHERE id = $1", guest_id)
        .fetch_one(&state.db)
        .await;
    
    if let Ok(r) = row {
        let new_val = !r.is_sent.unwrap_or(false);
        let _ = sqlx::query!("UPDATE guests SET is_sent = $1 WHERE id = $2", new_val, guest_id)
            .execute(&state.db)
            .await;
        axum::Json(serde_json::json!({ "status": "ok", "is_sent": new_val })).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Guest not found").into_response()
    }
}

#[derive(Deserialize)]
pub struct BulkAddGuestsRequest {
    pub names: String,
    pub category: Option<String>,
}

pub async fn bulk_add_guests(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Form(payload): Form<BulkAddGuestsRequest>,
) -> impl IntoResponse {
    let user_id = if let Some(cookie) = jar.get("user_id") {
        Uuid::parse_str(cookie.value()).ok()
    } else { None };

    if user_id.is_none() { return Redirect::to("/").into_response(); }

    let (invitation_id, _plan_name): (Uuid, Option<String>) = sqlx::query_as("SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2 AND deleted_at IS NULL")
        .bind(&slug)
        .bind(user_id.unwrap())
        .fetch_one(&state.db)
        .await
        .unwrap();

    let names_list: Vec<&str> = payload.names
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut tx = state.db.begin().await.unwrap();

    for name in names_list {
        let guest_slug = name.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join("-");
        
        let _ = sqlx::query(
            "INSERT INTO guests (invitation_id, name, category, slug, template_override, ai_language, song_id) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(invitation_id)
        .bind(name)
        .bind(&payload.category)
        .bind(&guest_slug)
        .bind(&None::<String>)
        .bind(&"".to_string())
        .bind(&None::<Uuid>)
        .execute(&mut *tx)
        .await;
    }

    tx.commit().await.unwrap();

    invalidate_invitation_cache(&state, &slug).await;
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

    invalidate_invitation_cache(&state, &slug).await;
    Redirect::to(&format!("/invitation/{}/manage#rsvps", slug)).into_response()
}

#[derive(Deserialize)]
pub struct AddGroupRequest {
    pub name: String,
    pub template_name: String,
    #[serde(default)]
    pub ai_language: Option<String>,
    #[serde(default)]
    pub song_id: Option<Uuid>,
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
        "SELECT id, plan_name FROM invitations WHERE slug = $1 AND user_id = $2 AND deleted_at IS NULL"
    )
    .bind(&slug)
    .bind(user_id.unwrap())
    .fetch_one(&state.db)
    .await
    .unwrap();

    let plan_name = plan_name.unwrap_or_else(|| "NOBLE".to_string());

    // Check if group already exists (for updates)
    let existing = sqlx::query("SELECT id FROM invitation_groups WHERE invitation_id = $1 AND name = $2")
        .bind(invitation_id)
        .bind(&payload.name)
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
        "INSERT INTO invitation_groups (invitation_id, name, template_name, ai_language, song_id) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (invitation_id, name) DO UPDATE SET template_name = $3, ai_language = $4, song_id = $5"
    )
    .bind(invitation_id)
    .bind(&payload.name)
    .bind(&payload.template_name)
    .bind(&ai_language)
    .bind(&payload.song_id)
    .execute(&state.db)
    .await
    .unwrap();

    invalidate_invitation_cache(&state, &slug).await;
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
        "UPDATE invitation_groups SET name = $1, template_name = $2, ai_language = $3, song_id = $4 WHERE id = $5"
    )
    .bind(&payload.name)
    .bind(&payload.template_name)
    .bind(&payload.ai_language)
    .bind(&payload.song_id)
    .bind(group_id)
    .execute(&state.db)
    .await
    .unwrap();

    invalidate_invitation_cache(&state, &slug).await;
    Redirect::to(&format!("/invitation/{}/manage#groups", slug)).into_response()
}



pub async fn delete_group(
    Path((slug, group_id)): Path<(String, Uuid)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    sqlx::query("DELETE FROM invitation_groups WHERE id = $1").bind(group_id).execute(&state.db).await.unwrap();
    invalidate_invitation_cache(&state, &slug).await;
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
    pub average_order_value: i64,
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
    let avg_order = if successful_count > 0 { (total_revenue as f64 / successful_count as f64).round() as i64 } else { 0 };

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

    let current_plan_name = invitation.plan_name.as_deref().unwrap_or("NOBLE");
    let target_plan_name = payload.target_plan.as_str();
    
    let current_plan = sqlx::query_as::<_, Plan>("SELECT * FROM plans WHERE code = $1")
        .bind(current_plan_name)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
        
    let target_plan = sqlx::query_as::<_, Plan>("SELECT * FROM plans WHERE code = $1")
        .bind(target_plan_name)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let current_plan_price = match current_plan {
        Some(p) => p.price,
        None => 50000,
    };

    let target_plan_price = match target_plan {
        Some(p) => p.price,
        None => match target_plan_name {
            "ROYAL" => 100000,
            "DYNASTY" => 300000,
            _ => 50000,
        },
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

                    // First get the pending booking to see if it has a voucher_code
                    let booking = sqlx::query_as::<_, Booking>(
                        "SELECT * FROM bookings WHERE invitation_id = (SELECT id FROM invitations WHERE slug = $1) AND status = 'PENDING' LIMIT 1"
                    )
                    .bind(&slug)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);

                    // Also try to update booking status using slug as fallback if ID fails later
                    let _ = sqlx::query("UPDATE bookings SET status = 'SUCCESS', updated_at = NOW() WHERE invitation_id = (SELECT id FROM invitations WHERE slug = $1)")
                        .bind(&slug)
                        .execute(&state.db)
                        .await;

                    // If we had a pending booking that just got successful, and it had a voucher code, increment usage count
                    if let Some(b) = booking {
                        if let Some(code) = b.voucher_code {
                            // Try incrementing referral usage
                            let _ = sqlx::query("UPDATE referrals SET usage_count = usage_count + 1 WHERE code = $1")
                                .bind(&code)
                                .execute(&state.db)
                                .await;
                                
                            // Try incrementing voucher usage
                            let _ = sqlx::query("UPDATE vouchers SET usage_count = usage_count + 1 WHERE code = $1")
                                .bind(&code)
                                .execute(&state.db)
                                .await;
                        }
                    }

                    // Send Email Notification
                    #[derive(sqlx::FromRow)]
                    struct PaymentUserInfo {
                        email: String,
                        user_name: Option<String>,
                        language: Option<String>,
                    }

                    let user_info = sqlx::query_as::<_, PaymentUserInfo>(
                         "SELECT u.email, u.name as user_name, i.language 
                          FROM invitations i 
                          JOIN users u ON i.user_id = u.id 
                          WHERE i.slug = $1"
                    )
                    .bind(&slug)
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

            // First get the pending booking to check for voucher_code
            let booking = sqlx::query_as::<_, Booking>(
                "SELECT * FROM bookings WHERE (invoice_id = $1 OR invoice_id = $2 OR payment_link LIKE $3) AND status = 'PENDING' LIMIT 1"
            )
            .bind(id)
            .bind(id_from_link.unwrap_or(""))
            .bind(format!("%{}%", id))
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

            // Match by invoice_id OR try to find by payment_link if it contains the ID
            let result = sqlx::query("UPDATE bookings SET status = $1, updated_at = NOW() WHERE invoice_id = $2 OR invoice_id = $3 OR payment_link LIKE $4")
                .bind(status)
                .bind(id)
                .bind(id_from_link.unwrap_or(""))
                .bind(format!("%{}%", id))
                .execute(&state.db)
                .await;
                
            if status == "SUCCESS" {
                if let Some(b) = booking {
                    if let Some(code) = b.voucher_code {
                        // Try incrementing referral usage
                        let _ = sqlx::query("UPDATE referrals SET usage_count = usage_count + 1 WHERE code = $1")
                            .bind(&code)
                            .execute(&state.db)
                            .await;
                            
                        // Try incrementing voucher usage
                        let _ = sqlx::query("UPDATE vouchers SET usage_count = usage_count + 1 WHERE code = $1")
                            .bind(&code)
                            .execute(&state.db)
                            .await;
                    }
                }
            }
            
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
    // If it contains 'T', extract the date part
    let clean_date = if date_str.contains('T') {
        date_str.split('T').next().unwrap_or(date_str)
    } else {
        date_str
    };

    // Handle YYYY-MM-DD
    if clean_date.contains('-') && clean_date.len() == 10 {
        let parts: Vec<&str> = clean_date.split('-').collect();
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
    }

    // If it is just a day number followed by T (e.g. "16T08:00:00")
    if date_str.contains('T') {
        let parts: Vec<&str> = date_str.split('T').collect();
        if !parts[0].is_empty() && parts[0].chars().all(char::is_numeric) {
            return parts[0].to_string();
        }
    }

    date_str.to_string()
}

// --- Admin Template Management ---

#[derive(Template)]
#[template(path = "admin/templates.html")]
pub struct AdminTemplatesTemplate {
    pub user: Option<User>,
    pub templates: Vec<InvitationTemplate>,
    pub is_dev: bool,
    pub current_page: i32,
    pub total_pages: i32,
    pub total_count: i64,
    pub published_count: i64,
    pub draft_count: i64,
    pub pagination_range: Vec<String>,
    pub search: String,
    pub category: String,
    pub status_filter: String,
    pub sort: String,
}

#[derive(Template)]
#[template(path = "admin/template_form.html")]
pub struct AdminTemplateFormTemplate {
    pub user: Option<User>,
    pub template: Option<InvitationTemplate>,
    pub is_dev: bool,
}

pub async fn admin_templates(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    let page = params.get("page").and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
    let per_page = 10;
    let offset = (page - 1) * per_page;

    let search = params.get("search").cloned().unwrap_or_default();
    let category = params.get("category").cloned().unwrap_or_default();
    let status_filter = params.get("status").cloned().unwrap_or_default();
    let sort = params.get("sort").cloned().unwrap_or_else(|| "featured".to_string());

    // 1. Fetch filtered templates using QueryBuilder
    let mut qb = sqlx::QueryBuilder::new("SELECT * FROM templates WHERE TRUE");
    
    if !search.is_empty() {
        qb.push(" AND (title ILIKE ");
        qb.push_bind(format!("%{}%", search));
        qb.push(" OR id ILIKE ");
        qb.push_bind(format!("%{}%", search));
        qb.push(")");
    }
    if !category.is_empty() && category != "All" {
        qb.push(" AND category = ");
        qb.push_bind(&category);
    }
    if !status_filter.is_empty() && status_filter != "All" {
        qb.push(" AND status = ");
        qb.push_bind(&status_filter);
    }

    // Sort order
    qb.push(" ORDER BY ");
    match sort.as_str() {
        "oldest" => qb.push("created_at ASC"),
        "title_asc" => qb.push("title ASC"),
        "title_desc" => qb.push("title DESC"),
        "featured" => qb.push("is_featured DESC, created_at DESC"),
        _ => qb.push("created_at DESC"),
    };

    // Pagination
    qb.push(" LIMIT ");
    qb.push_bind(per_page as i64);
    qb.push(" OFFSET ");
    qb.push_bind(offset as i64);

    let templates: Vec<InvitationTemplate> = qb.build_query_as::<InvitationTemplate>()
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(sanitize_template_urls)
        .collect();

    // 2. Count filtered results for pagination
    let mut count_qb = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM templates WHERE TRUE");
    if !search.is_empty() {
        count_qb.push(" AND (title ILIKE ");
        count_qb.push_bind(format!("%{}%", search));
        count_qb.push(" OR id ILIKE ");
        count_qb.push_bind(format!("%{}%", search));
        count_qb.push(")");
    }
    if !category.is_empty() && category != "All" {
        count_qb.push(" AND category = ");
        count_qb.push_bind(&category);
    }
    if !status_filter.is_empty() && status_filter != "All" {
        count_qb.push(" AND status = ");
        count_qb.push_bind(&status_filter);
    }

    let total_count: i64 = count_qb.build_query_scalar()
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    // 3. Overall Stats (unfiltered)
    let stats: (i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COUNT(*) FILTER (WHERE status = 'PUBLISHED'), COUNT(*) FILTER (WHERE status = 'DRAFT') FROM templates"
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0, 0));
    
    let published_count = stats.1;
    let draft_count = stats.2;
    let total_pages = (total_count as f64 / per_page as f64).ceil() as i32;

    // Smart Pagination Range (e.g. 1, 2, ..., 5, 6, 7, ..., 18)
    let mut pagination_range = Vec::new();
    if total_pages <= 7 {
        for i in 1..=total_pages {
            pagination_range.push(i.to_string());
        }
    } else {
        pagination_range.push("1".to_string());
        
        let start = if page > 3 { page - 1 } else { 2 };
        let end = if page < total_pages - 2 { page + 1 } else { total_pages - 1 };
        
        if start > 2 {
            pagination_range.push("...".to_string());
        }
        
        for i in start..=end {
            pagination_range.push(i.to_string());
        }
        
        if end < total_pages - 1 {
            pagination_range.push("...".to_string());
        }
        
        pagination_range.push(total_pages.to_string());
    }

    HtmlTemplate(AdminTemplatesTemplate {
        user,
        templates,
        is_dev: state.is_dev,
        current_page: page,
        total_pages,
        total_count,
        published_count,
        draft_count,
        pagination_range,
        search,
        category,
        status_filter,
        sort,
    }).into_response()
}

pub async fn admin_templates_new(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    HtmlTemplate(AdminTemplateFormTemplate {
        user,
        template: None,
        is_dev: state.is_dev,
    }).into_response()
}

pub async fn admin_templates_create(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    let mut id = String::new();
    let mut slug = String::new();
    let mut title = String::new();
    let mut desc = String::new();
    let mut category = String::new();
    let mut preview_img = String::new();
    let mut preview_video: Option<String> = None;
    let mut status = String::new();
    let mut is_featured = false;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "id" => id = field.text().await.unwrap_or_default(),
            "slug" => slug = field.text().await.unwrap_or_default(),
            "title" => title = field.text().await.unwrap_or_default(),
            "desc" => desc = field.text().await.unwrap_or_default(),
            "category" => category = field.text().await.unwrap_or_default(),
            "preview_img" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    preview_img = rewrite_s3_url_to_proxy(&text);
                }
            },
            "preview_video" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    preview_video = Some(rewrite_s3_url_to_proxy(&text));
                }
            },
            "status" => status = field.text().await.unwrap_or_default(),
            "is_featured" => is_featured = true,
            "preview_file" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    if !file_name.is_empty() {
                        let data = field.bytes().await.unwrap_or_default();
                        if !data.is_empty() {
                            let save_name = if !id.is_empty() { id.clone() } else { Uuid::new_v4().to_string() };
                            let path = format!("static/uploads/{}.webp", save_name);
                            if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1200).await {
                                preview_img = format!("/uploads/{}.webp", save_name);
                            }
                        }
                    }
                }
            },
            "preview_video_file" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    if !file_name.is_empty() {
                        let data = field.bytes().await.unwrap_or_default();
                        if !data.is_empty() {
                            let save_name = if !id.is_empty() { id.clone() } else { Uuid::new_v4().to_string() };
                            let path = format!("static/uploads/{}_preview.mp4", save_name);
                            if let Some(actual_path) = compress_and_save_video(&state.s3_client, &state.s3_bucket, data, path.clone()).await {
                                preview_video = Some(actual_path);
                            }
                        }
                    }
                }
            },
            _ => {}
        }
    }

    let res = sqlx::query(
        "INSERT INTO templates (id, slug, title, description, category, preview_img, preview_video, status, is_featured) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(&id)
    .bind(&slug)
    .bind(&title)
    .bind(&desc)
    .bind(&category)
    .bind(&preview_img)
    .bind(&preview_video)
    .bind(&status)
    .bind(is_featured)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => Redirect::to("/admin/templates").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create template: {}", e)).into_response(),
    }
}

pub async fn admin_templates_edit(
    State(state): State<AppState>,
    Path(id): Path<String>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    let template = sqlx::query_as::<_, InvitationTemplate>("SELECT * FROM templates WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
        .map(sanitize_template_urls);

    if let Some(t) = template {
        HtmlTemplate(AdminTemplateFormTemplate {
            user,
            template: Some(t),
            is_dev: state.is_dev,
        }).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Template not found").into_response()
    }
}

pub async fn admin_templates_update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    jar: PrivateCookieJar,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    let mut slug = String::new();
    let mut title = String::new();
    let mut desc = String::new();
    let mut category = String::new();
    let mut preview_img = String::new();
    let mut preview_video: Option<String> = None;
    let mut status = String::new();
    let mut is_featured = false;

    // First fetch current template to get existing preview_img and preview_video as fallback
    let current: Option<(String, Option<String>)> = sqlx::query_as("SELECT preview_img, preview_video FROM templates WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
    
    let mut old_preview_video = None;
    if let Some((c_img, c_vid)) = current {
        preview_img = c_img;
        preview_video = c_vid.clone();
        old_preview_video = c_vid;
    }

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "slug" => slug = field.text().await.unwrap_or_default(),
            "title" => title = field.text().await.unwrap_or_default(),
            "desc" => desc = field.text().await.unwrap_or_default(),
            "category" => category = field.text().await.unwrap_or_default(),
            "preview_img" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    preview_img = rewrite_s3_url_to_proxy(&text);
                }
            },
            "preview_video" => {
                let text = field.text().await.unwrap_or_default();
                if text.is_empty() {
                    preview_video = None;
                } else {
                    preview_video = Some(rewrite_s3_url_to_proxy(&text));
                }
            },
            "status" => status = field.text().await.unwrap_or_default(),
            "is_featured" => is_featured = true,
            "preview_file" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    if !file_name.is_empty() {
                        let data = field.bytes().await.unwrap_or_default();
                        if !data.is_empty() {
                            let path = format!("static/uploads/{}.webp", id);
                            if process_and_save_image(&state.s3_client, &state.s3_bucket, data, path.clone(), 1200).await {
                                preview_img = format!("/uploads/{}.webp", id);
                            }
                        }
                    }
                }
            },
            "preview_video_file" => {
                if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
                    if !file_name.is_empty() {
                        let data = field.bytes().await.unwrap_or_default();
                        if !data.is_empty() {
                            let path = format!("static/uploads/{}_preview.mp4", id);
                            if let Some(actual_path) = compress_and_save_video(&state.s3_client, &state.s3_bucket, data, path.clone()).await {
                                preview_video = Some(actual_path);
                            }
                        }
                    }
                }
            },
            _ => {}
        }
    }

    let res = sqlx::query(
        "UPDATE templates SET slug = $1, title = $2, description = $3, category = $4, preview_img = $5, preview_video = $6, status = $7, is_featured = $8, updated_at = NOW() WHERE id = $9"
    )
    .bind(&slug)
    .bind(&title)
    .bind(&desc)
    .bind(&category)
    .bind(&preview_img)
    .bind(&preview_video)
    .bind(&status)
    .bind(is_featured)
    .bind(&id)
    .execute(&state.db)
    .await;

    if res.is_ok() {
        if let Some(ref old_vid) = old_preview_video {
            if preview_video.as_ref() != Some(old_vid) {
                delete_s3_file(&state.s3_client, &state.s3_bucket, old_vid).await;
            }
        }
    }

    match res {
        Ok(_) => Redirect::to("/admin/templates").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update template: {}", e)).into_response(),
    }
}

pub async fn admin_templates_toggle_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    // Check if it's featured
    #[derive(sqlx::FromRow)]
    struct TemplateStatusInfo {
        status: String,
        is_featured: bool,
    }

    let template = sqlx::query_as::<_, TemplateStatusInfo>("SELECT status, is_featured FROM templates WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    if let Some(t) = template {
        if t.is_featured && t.status == "PUBLISHED" {
            // Cannot toggle to DRAFT if featured
            return Redirect::to("/admin/templates").into_response();
        }
    }

    let res = sqlx::query(
        "UPDATE templates SET status = CASE WHEN status = 'PUBLISHED' THEN 'DRAFT' ELSE 'PUBLISHED' END, updated_at = NOW() WHERE id = $1"
    )
    .bind(&id)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => Redirect::to("/admin/templates").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to toggle status: {}", e)).into_response(),
    }
}

pub async fn admin_templates_toggle_featured(
    State(state): State<AppState>,
    Path(id): Path<String>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    let res = sqlx::query(
        "UPDATE templates SET is_featured = NOT is_featured, updated_at = NOW() WHERE id = $1"
    )
    .bind(&id)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => Redirect::to("/admin/templates").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to toggle featured: {}", e)).into_response(),
    }
}

pub async fn admin_templates_delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return (StatusCode::FORBIDDEN, "Admin access required").into_response();
    }

    let res = sqlx::query("DELETE FROM templates WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await;

    match res {
        Ok(_) => Redirect::to("/admin/templates").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete template: {}", e)).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ValidatePromoQuery {
    pub code: String,
}

pub async fn validate_promo(
    State(state): State<AppState>,
    Query(query): Query<ValidatePromoQuery>,
) -> impl IntoResponse {
    let code = query.code.trim().to_string();
    
    // Check referrals first
    let referral = sqlx::query_as::<_, Referral>("SELECT * FROM referrals WHERE code = $1 AND is_active = true")
        .bind(&code)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
        
    if let Some(r) = referral {
        return Json(json!({
            "valid": true,
            "type": "referral",
            "discount_percent": r.discount_percent
        })).into_response();
    }
    
    // Check vouchers
    let voucher = sqlx::query_as::<_, Voucher>("SELECT * FROM vouchers WHERE code = $1 AND is_active = true AND (usage_limit IS NULL OR usage_count < usage_limit) AND (valid_until IS NULL OR valid_until > NOW())")
        .bind(&code)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
        
    if let Some(v) = voucher {
        return Json(json!({
            "valid": true,
            "type": "voucher",
            "discount_percent": v.discount_percent
        })).into_response();
    }
    
    (StatusCode::BAD_REQUEST, Json(json!({
        "valid": false,
        "error": "Kode promo tidak valid atau sudah kadaluarsa"
    }))).into_response()
}

#[derive(Template)]
#[template(path = "admin/marketing/dashboard.html")]
pub struct AdminMarketingTemplate {
    #[allow(dead_code)]
    pub user: Option<User>,
    pub plans: Vec<Plan>,
    pub vouchers: Vec<Voucher>,
    pub referrals: Vec<Referral>,
}

pub async fn admin_marketing(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    if !is_superadmin(&user) {
        return Redirect::to("/").into_response();
    }
    
    let plans = sqlx::query_as::<_, Plan>("SELECT * FROM plans ORDER BY price ASC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        
    let vouchers = sqlx::query_as::<_, Voucher>("SELECT * FROM vouchers ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        
    let referrals = sqlx::query_as::<_, Referral>("SELECT * FROM referrals ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    HtmlTemplate(AdminMarketingTemplate {
        user,
        plans,
        vouchers,
        referrals,
    }).into_response()
}

// --- Helpers ---

async fn get_user_from_jar(db: &sqlx::PgPool, jar: &PrivateCookieJar) -> Option<User> {
    if let Some(cookie) = jar.get("user_id") {
        let uid = Uuid::parse_str(cookie.value()).ok();
        if let Some(id) = uid {
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await
                .unwrap_or(None)
        } else { None }
    } else { None }
}

fn is_superadmin(user: &Option<User>) -> bool {
    if let Some(u) = user {
        u.role == "SUPERADMIN"
    } else {
        false
    }
}


pub fn render_invitation_template(template_name: &str, invitation: Invitation, is_dev: bool) -> axum::response::Response {
    match template_name {
                "keraton-dark-invitation" => HtmlTemplate(KeratonDarkInvitationTemplate { invitation, is_dev: is_dev }).into_response(),
                "loveanthem" => HtmlTemplate(LoveAnthemTemplate { invitation, is_dev: is_dev }).into_response(),
                "cinemarry" => HtmlTemplate(CineMarryTemplate { invitation, is_dev: is_dev }).into_response(),
                "super-wedbros" => HtmlTemplate(SuperWedbrosTemplate { invitation, is_dev: is_dev }).into_response(),
                "royal-heritage" => HtmlTemplate(RoyalHeritageTemplate { invitation, is_dev: is_dev }).into_response(),
                "high-fashion-editorial" => HtmlTemplate(HighFashionEditorialTemplate { invitation: invitation.clone(), is_dev: is_dev }).into_response(),
                "reel-wedding" => HtmlTemplate(ReelWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "cairide" => HtmlTemplate(CaiRideTemplate { invitation, is_dev: is_dev }).into_response(),
                "pinterlove" => HtmlTemplate(PinterLoveTemplate { invitation, is_dev: is_dev }).into_response(),
                "shopee-live-wedding" => HtmlTemplate(ShopeeLiveWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "tiktok-live-wedding" => HtmlTemplate(TiktokLiveWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-uber" => HtmlTemplate(WeUberTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-disney" => HtmlTemplate(WeddingDisneyTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-facebook" => HtmlTemplate(WeddingFacebookTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-iphone-theme" => HtmlTemplate(WeddingIphoneThemeTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-netflix-v2" => HtmlTemplate(WeddingNetflixV2Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-prime" => HtmlTemplate(WeddingPrimeTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-wrath-v2" => HtmlTemplate(WeddingWrathV2Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-applemusic" => HtmlTemplate(AppleMusicTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-capcut" => HtmlTemplate(WeCapCutTemplate { invitation, is_dev: is_dev }).into_response(),
                "bereal-wedding" => HtmlTemplate(BeRealWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "instagram-live-wedding" => HtmlTemplate(InstagramLiveWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-discord" => HtmlTemplate(WeDiscordTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-webtoon" => HtmlTemplate(WeWebtoonTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-mixue" => HtmlTemplate(WeMixueTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-playstation" => HtmlTemplate(WePlayStationTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-threads-app" => HtmlTemplate(WeThreadsAppTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-alfamart" => HtmlTemplate(WeddingAlfamartTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-kai" => HtmlTemplate(WeddingKaiTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-medium" => HtmlTemplate(WeddingMediumTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-transjakarta" => HtmlTemplate(WeddingTransJakartaTemplate { invitation, is_dev: is_dev }).into_response(),
                "qris-wedding" => HtmlTemplate(QrisWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-grab" => HtmlTemplate(WeddingGrabTemplate { invitation, is_dev: is_dev }).into_response(),
                "figma-wedding" => HtmlTemplate(FigmaWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-whatsapp-theme" => HtmlTemplate(WeddingWhatsappThemeTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-manga" => HtmlTemplate(WeMangaTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-nintendo-switch" => HtmlTemplate(WeNintendoSwitchTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-kai-v2" => HtmlTemplate(WeddingKaiV2Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-minecraft" => HtmlTemplate(WeddingMinecraftTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-zoom-v2" => HtmlTemplate(WeddingZoomV2Template { invitation, is_dev: is_dev }).into_response(),
                "we-vscode" => HtmlTemplate(WeVSCodeTemplate { invitation, is_dev: is_dev }).into_response(),
                "gmail-wedding" => HtmlTemplate(GmailWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-behance" => HtmlTemplate(WeBehanceTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-chatime" => HtmlTemplate(WeChatimeTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-dribbble" => HtmlTemplate(WeDribbbleTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-hm" => HtmlTemplate(WeHMTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-janjijiwa" => HtmlTemplate(WeJanjiJiwaTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-kopikenangan" => HtmlTemplate(WeKopiKenanganTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-powerpoint" => HtmlTemplate(WePowerPointTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-talenta" => HtmlTemplate(WeTalentaTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-animal-crossing" => HtmlTemplate(WeddingAnimalCrossingTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-claude" => HtmlTemplate(WeddingClaudeTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-cod" => HtmlTemplate(WeddingCodTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-danamon" => HtmlTemplate(WeddingDanamonTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-excel-theme" => HtmlTemplate(WeddingExcelThemeTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-freefire" => HtmlTemplate(WeddingFreeFireTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-github" => HtmlTemplate(WeddingGithubTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-jenius-v2" => HtmlTemplate(WeddingJeniusV2Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-linux" => HtmlTemplate(WeddingLinuxTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-word-theme" => HtmlTemplate(WeddingWordThemeTemplate { invitation, is_dev: is_dev }).into_response(),
                "canva-elegant-wedding" => HtmlTemplate(CanvaElegantWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "elegant-wedding" => HtmlTemplate(ElegantWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "mrt-wedding" => HtmlTemplate(MrtWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-brimo" => HtmlTemplate(WeBrimoTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-duolingo" => HtmlTemplate(WeDuolingoTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-google-calendar" => HtmlTemplate(WeGoogleCalendarTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-livin" => HtmlTemplate(WeLivinTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-manhua" => HtmlTemplate(WeManhuaTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-manhwa" => HtmlTemplate(WeManhwaTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-momoyo" => HtmlTemplate(WeMomoyoTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-steam-store" => HtmlTemplate(WeSteamStoreTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-uniqlo" => HtmlTemplate(WeUniqloTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-zara" => HtmlTemplate(WeZaraTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-bpjs" => HtmlTemplate(WeddingBpjsTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-chatgpt" => HtmlTemplate(WeddingChatGptTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-familymart" => HtmlTemplate(WeddingFamilyMartTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-gemini" => HtmlTemplate(WeddingGeminiTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-genshin-theme" => HtmlTemplate(WeddingGenshinThemeTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-indomaret" => HtmlTemplate(WeddingIndomaretTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-jago" => HtmlTemplate(WeddingJagoTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-macintosh" => HtmlTemplate(WeddingMacintoshTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-mlbb" => HtmlTemplate(WeddingMlbbTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-ps5" => HtmlTemplate(WeddingPs5Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-pubg" => HtmlTemplate(WeddingPubgTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-telegram-theme" => HtmlTemplate(WeddingTelegramThemeTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-wa-channel" => HtmlTemplate(WeddingWaChannelTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-windows95" => HtmlTemplate(WeddingWindows95Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-windowsxp" => HtmlTemplate(WeddingWindowsXpTemplate { invitation, is_dev: is_dev }).into_response(),
                "whoosh-wedding" => HtmlTemplate(WhooshWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "absensi-wedding" => HtmlTemplate(AbsensiWeddingTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-asana" => HtmlTemplate(WeAsanaTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-kopijago" => HtmlTemplate(WeKopiJagoTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-linktree" => HtmlTemplate(WeLinktreeTemplate { invitation, is_dev: is_dev }).into_response(),
                "we-upwork" => HtmlTemplate(WeUpworkTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-danantara" => HtmlTemplate(WeddingDanantaraTemplate { invitation, is_dev: is_dev }).into_response(),
                "wedding-dota2" => HtmlTemplate(WeddingDota2Template { invitation, is_dev: is_dev }).into_response(),
                "wedding-indomie-goreng" => HtmlTemplate(WeddingIndomieGorengTemplate { invitation, is_dev: is_dev }).into_response(),
                "trendvibe" => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: is_dev }).into_response(),
                _ => HtmlTemplate(TrendVibeTemplate { invitation, is_dev: is_dev }).into_response(),
            }
}


pub async fn invalidate_invitation_cache(state: &AppState, slug: &str) {
    if let Ok(mut conn) = state.redis.get().await {
        let _ = redis::cmd("DEL").arg(format!("invitation_cache:{}", slug)).query_async::<()>(&mut conn).await;
    }
}

// SEO Landing Page Templates
#[derive(Template)]
#[template(path = "landing/undangan_digital.html")]
pub struct SeoUndanganDigitalTemplate {
    pub user: Option<User>,
    #[allow(dead_code)]
    pub is_dev: bool,
    pub active_category: String,
    pub templates: Vec<InvitationTemplate>,
    pub current_page: i32,
    pub total_pages: i32,
    pub search_query: String,
    pub sort: String,
}

#[derive(Template)]
#[template(path = "landing/undangan_pernikahan.html")]
pub struct SeoUndanganPernikahanTemplate {
    pub user: Option<User>,
    #[allow(dead_code)]
    pub is_dev: bool,
    pub active_category: String,
    pub templates: Vec<InvitationTemplate>,
    pub current_page: i32,
    pub total_pages: i32,
    pub search_query: String,
    pub sort: String,
}

#[derive(Template)]
#[template(path = "landing/undangan_digital_ai.html")]
pub struct SeoUndanganDigitalAiTemplate {
    pub user: Option<User>,
    #[allow(dead_code)]
    pub is_dev: bool,
    pub active_category: String,
    pub templates: Vec<InvitationTemplate>,
    pub current_page: i32,
    pub total_pages: i32,
    pub search_query: String,
    pub sort: String,
}

#[derive(Template)]
#[template(path = "landing/undangan_banyak_template_dalam_satu_undangan.html")]
pub struct SeoUndanganBanyakTemplate {
    pub user: Option<User>,
    #[allow(dead_code)]
    pub is_dev: bool,
    pub active_category: String,
    pub templates: Vec<InvitationTemplate>,
    pub current_page: i32,
    pub total_pages: i32,
    pub search_query: String,
    pub sort: String,
}

// Blog Templates
#[derive(Template)]
#[template(path = "blog/index.html")]
pub struct BlogIndexTemplate {
    pub user: Option<User>,
    pub posts: Vec<BlogPost>,
    #[allow(dead_code)]
    pub is_dev: bool,
}

#[derive(Template)]
#[template(path = "blog/detail.html")]
pub struct BlogDetailTemplate {
    pub user: Option<User>,
    pub post: BlogPost,
    #[allow(dead_code)]
    pub is_dev: bool,
}

// SEO Handlers
pub async fn seo_undangan_digital(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    let (active_category, templates, current_page, total_pages, search_query, sort) = get_paginated_templates(&state.db, &params).await;
    HtmlTemplate(SeoUndanganDigitalTemplate { user, is_dev: state.is_dev, active_category, templates, current_page, total_pages, search_query, sort })
}

pub async fn seo_undangan_pernikahan(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    let (active_category, templates, current_page, total_pages, search_query, sort) = get_paginated_templates(&state.db, &params).await;
    HtmlTemplate(SeoUndanganPernikahanTemplate { user, is_dev: state.is_dev, active_category, templates, current_page, total_pages, search_query, sort })
}

pub async fn seo_undangan_digital_ai(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    let (active_category, templates, current_page, total_pages, search_query, sort) = get_paginated_templates(&state.db, &params).await;
    HtmlTemplate(SeoUndanganDigitalAiTemplate { user, is_dev: state.is_dev, active_category, templates, current_page, total_pages, search_query, sort })
}

pub async fn seo_undangan_banyak_template(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    let (active_category, templates, current_page, total_pages, search_query, sort) = get_paginated_templates(&state.db, &params).await;
    HtmlTemplate(SeoUndanganBanyakTemplate { user, is_dev: state.is_dev, active_category, templates, current_page, total_pages, search_query, sort })
}

// Blog Handlers
pub async fn blog_index(State(state): State<AppState>, jar: PrivateCookieJar) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    let posts = sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE is_published = true ORDER BY published_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    HtmlTemplate(BlogIndexTemplate { user, posts, is_dev: state.is_dev })
}

pub async fn blog_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let user = get_user_from_jar(&state.db, &jar).await;
    let post = sqlx::query_as::<_, BlogPost>(
        "SELECT * FROM blog_posts WHERE slug = $1 AND is_published = true"
    )
    .bind(slug)
    .fetch_optional(&state.db)
    .await
    .unwrap_or_default();

    match post {
        Some(p) => HtmlTemplate(BlogDetailTemplate { user, post: p, is_dev: state.is_dev }).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "Blog post not found").into_response(),
    }
}
