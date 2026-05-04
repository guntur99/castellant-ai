use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use askama::Template;
use std::env;

#[derive(Template, Clone)]
#[template(path = "email/payment_success.html")]
pub struct PaymentSuccessEmail {
    pub name: String,
    pub plan_name: String,
    pub slug: String,
    pub amount: i32,
    pub language: String,
    pub base_url: String,
}

pub async fn send_payment_success_email(to_email: &str, template: PaymentSuccessEmail, port_override: Option<u16>) -> Result<(), String> {
    // 1. Try Mailtrap API first if Token is present
    if let Ok(token) = env::var("MAILTRAP_API_TOKEN") {
        if !token.is_empty() {
            let from_email = env::var("MAIL_FROM_ADDRESS").unwrap_or_else(|_| "no-reply@example.com".to_string());
            let from_name = env::var("MAIL_FROM_NAME").unwrap_or_else(|_| "Castellant Team".to_string());
            let email_body = template.render().map_err(|e| format!("Failed to render email: {}", e))?;
            let language = template.language.clone();
            let subject = if language == "id" {
                "Pembayaran Berhasil! Undangan Anda Sudah Siap ✨"
            } else {
                "Payment Confirmed! Your Invitation is Ready ✨"
            };

            let client = reqwest::Client::new();
            let payload = serde_json::json!({
                "from": {"email": from_email, "name": from_name},
                "to": [{"email": to_email}],
                "subject": subject,
                "html": email_body
            });

            match client.post("https://send.api.mailtrap.io/api/send")
                .header("Api-Token", token)
                .json(&payload)
                .send()
                .await {
                    Ok(res) => {
                        if res.status().is_success() {
                            return Ok(());
                        } else {
                            let status = res.status();
                            let err_text = res.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            tracing::warn!("Mailtrap API failed (status {}): {}. Falling back to SMTP...", status, err_text);
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Mailtrap API connection error: {}. Falling back to SMTP...", e);
                    }
                }
        }
    }

    // 2. Fallback to SMTP
    let smtp_host = env::var("MAIL_HOST").unwrap_or_else(|_| "localhost".to_string());
    let smtp_port = port_override.unwrap_or_else(|| env::var("MAIL_PORT").unwrap_or_else(|_| "2525".to_string()).parse::<u16>().unwrap_or(2525));
    let smtp_user = env::var("MAIL_USERNAME").unwrap_or_default();
    let smtp_pass = env::var("MAIL_PASSWORD").unwrap_or_default();
    let from_email = env::var("MAIL_FROM_ADDRESS").unwrap_or_else(|_| "no-reply@example.com".to_string());
    let from_name = env::var("MAIL_FROM_NAME").unwrap_or_else(|_| "Castellant Team".to_string());

    let language = template.language.clone();
    let email_body = template.render().map_err(|e| format!("Failed to render email: {}", e))?;

    let subject = if language == "id" {
        "Pembayaran Berhasil! Undangan Anda Sudah Siap ✨"
    } else {
        "Payment Confirmed! Your Invitation is Ready ✨"
    };

    let email = Message::builder()
        .from(format!("{} <{}>", from_name, from_email).parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject(subject)
        .header(lettre::message::header::ContentType::TEXT_HTML)
        .body(email_body)
        .map_err(|e| format!("Failed to build email: {}", e))?;

    let creds = Credentials::new(smtp_user, smtp_pass);

    let tls_parameters = lettre::transport::smtp::client::TlsParameters::builder(smtp_host.clone())
        .build()
        .map_err(|e| format!("Failed to build TlsParameters: {}", e))?;

    let tls = if smtp_port == 465 {
        lettre::transport::smtp::client::Tls::Wrapper(tls_parameters)
    } else {
        lettre::transport::smtp::client::Tls::Opportunistic(tls_parameters)
    };

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
        .map_err(|e| format!("Failed to create mailer: {}", e))?
        .port(smtp_port)
        .credentials(creds)
        .tls(tls)
        .timeout(Some(std::time::Duration::from_secs(30)))
        .build();

    mailer.send(email).await.map_err(|e| format!("Failed to send email: {}", e))?;

    Ok(())
}
