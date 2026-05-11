//! SMTP mailer.
//!
//! Configured via environment variables:
//!   SMTP_HOST      — default: 127.0.0.1      (mailpit in dev)
//!   SMTP_PORT      — default: 1025            (mailpit) / 465 when SMTP_TLS=true
//!   SMTP_TLS       — set to "true" to use TLS (required for Resend and other cloud SMTP)
//!   SMTP_FROM      — default: noreply@trictrac.local
//!   SMTP_USER      — optional SMTP credentials (use "resend" for Resend)
//!   SMTP_PASSWORD  — optional SMTP credentials (use Resend API key)
//!   APP_URL        — default: http://localhost:9091  (frontend base URL for email links)
//!
//! Production (Resend):
//!   SMTP_HOST=smtp.resend.com  SMTP_TLS=true
//!   SMTP_USER=resend  SMTP_PASSWORD=re_xxxx
//!   SMTP_FROM=noreply@yourdomain.com

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::Mailbox,
    transport::smtp::authentication::Credentials as SmtpCredentials,
};

pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    app_url: String,
}

impl Mailer {
    pub fn from_env() -> Self {
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let tls = std::env::var("SMTP_TLS").map(|v| v == "true").unwrap_or(false);
        let from_str = std::env::var("SMTP_FROM")
            .unwrap_or_else(|_| "noreply@trictrac.local".to_string());
        let app_url = std::env::var("APP_URL")
            .unwrap_or_else(|_| "http://localhost:9091".to_string());

        let credentials = if let (Ok(user), Ok(pass)) =
            (std::env::var("SMTP_USER"), std::env::var("SMTP_PASSWORD"))
        {
            Some(SmtpCredentials::new(user, pass))
        } else {
            None
        };

        let transport = if tls {
            // TLS on port 465 (Resend, SendGrid, etc.)
            let default_port = 465u16;
            let port: u16 = std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(default_port);
            let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
                .expect("invalid SMTP_HOST for TLS relay")
                .port(port);
            if let Some(creds) = credentials {
                builder = builder.credentials(creds);
            }
            builder.build()
        } else {
            // Plain SMTP (Mailpit dev, or local relay)
            let port: u16 = std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(1025);
            let mut builder =
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host).port(port);
            if let Some(creds) = credentials {
                builder = builder.credentials(creds);
            }
            builder.build()
        };

        let from = from_str
            .parse()
            .unwrap_or_else(|_| "noreply@trictrac.local".parse().unwrap());

        Self { transport, from, app_url }
    }

    pub async fn send_verification(&self, to_email: &str, token: &str) {
        let link = format!("{}/verify-email?token={}", self.app_url, token);
        let body = format!(
            "Welcome to Trictrac!\n\n\
             Please verify your email address by clicking the link below:\n\n\
             {link}\n\n\
             This link expires in 24 hours.\n"
        );
        self.send(to_email, "Verify your Trictrac account", body).await;
    }

    pub async fn send_password_reset(&self, to_email: &str, token: &str) {
        let link = format!("{}/reset-password?token={}", self.app_url, token);
        let body = format!(
            "You requested a password reset for your Trictrac account.\n\n\
             Click the link below to choose a new password:\n\n\
             {link}\n\n\
             This link expires in 1 hour.\n\
             If you did not request this, you can safely ignore this email.\n"
        );
        self.send(to_email, "Reset your Trictrac password", body).await;
    }

    async fn send(&self, to_email: &str, subject: &str, body: String) {
        let to: Mailbox = match to_email.parse() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("SMTP: invalid recipient address {to_email:?}: {e}");
                return;
            }
        };
        let email = match Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject)
            .body(body)
        {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("SMTP: failed to build message: {e}");
                return;
            }
        };
        if let Err(e) = self.transport.send(email).await {
            tracing::warn!("SMTP: send failed: {e}");
        }
    }
}
