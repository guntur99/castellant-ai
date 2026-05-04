use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let smtp_host = "sandbox.smtp.mailtrap.io";
    let smtp_port = 2525;
    let smtp_user = "5671b79a6f861a";
    let smtp_pass = "15c48939ab5ddb";
    let from_email = "team@castellant.com";
    let to_email = "gugunguntur99@gmail.com";

    println!("Testing SMTP connection to {} on port {}...", smtp_host, smtp_port);

    let email = Message::builder()
        .from(format!("Test <{}>", from_email).parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject("Test Email")
        .body("This is a test email from the script".to_string())
        .unwrap();

    let creds = Credentials::new(smtp_user.to_string(), smtp_pass.to_string());

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)
        .unwrap()
        .port(smtp_port)
        .credentials(creds)
        .timeout(Some(Duration::from_secs(10)))
        .build();

    match mailer.send(email).await {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => println!("Failed to send email: {}", e),
    }
}
