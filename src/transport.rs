//! SMTP transport: sending emails as `Io<EmailError, ()>`.
//!
//! Wraps lettre's `SmtpTransport` with effect-based APIs.
//! Connection lifecycle managed via `Resource`.

use comp_cat_rs::effect::io::Io;
use comp_cat_rs::effect::resource::Resource;
use lettre::Transport;

use crate::error::EmailError;
use crate::message::Email;

/// SMTP connection credentials.
#[derive(Debug, Clone)]
pub struct Credentials(lettre::transport::smtp::authentication::Credentials);

impl Credentials {
    /// Create credentials from username and password.
    #[must_use]
    pub fn new(username: String, password: String) -> Self {
        Self(lettre::transport::smtp::authentication::Credentials::new(username, password))
    }

    pub(crate) fn inner(&self) -> &lettre::transport::smtp::authentication::Credentials {
        &self.0
    }
}

/// SMTP server configuration.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    host: String,
    port: Option<u16>,
    credentials: Option<Credentials>,
    starttls: bool,
}

impl SmtpConfig {
    /// Configure for a given SMTP host.
    #[must_use]
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: None,
            credentials: None,
            starttls: true,
        }
    }

    /// Set the port (default: 587 for STARTTLS, 465 for implicit TLS).
    #[must_use]
    pub fn port(self, port: u16) -> Self {
        Self { port: Some(port), ..self }
    }

    /// Set authentication credentials.
    #[must_use]
    pub fn credentials(self, creds: Credentials) -> Self {
        Self { credentials: Some(creds), ..self }
    }

    /// Disable STARTTLS (use implicit TLS instead).
    #[must_use]
    pub fn no_starttls(self) -> Self {
        Self { starttls: false, ..self }
    }

    /// Common preset: Gmail SMTP.
    #[must_use]
    pub fn gmail(username: String, app_password: String) -> Self {
        Self::new("smtp.gmail.com")
            .port(587)
            .credentials(Credentials::new(username, app_password))
    }

    /// Common preset: Outlook/Office365 SMTP.
    #[must_use]
    pub fn outlook(username: String, password: String) -> Self {
        Self::new("smtp-mail.outlook.com")
            .port(587)
            .credentials(Credentials::new(username, password))
    }
}

/// Send a single email.
///
/// Opens an SMTP connection, sends, and closes within the `Io`.
///
/// # Errors
///
/// Returns `EmailError::Smtp` if the connection or send fails.
#[must_use]
pub fn send(config: SmtpConfig, email: Email) -> Io<EmailError, ()> {
    Io::suspend(move || {
        let transport = build_transport(&config)?;
        transport.send(email.inner())
            .map(|_| ())
            .map_err(EmailError::from)
    })
}

/// Send multiple emails over a single SMTP connection.
///
/// Opens the connection once, sends all messages, then closes.
///
/// # Errors
///
/// Returns the first `EmailError::Smtp` encountered.
/// Emails before the failure are sent; emails after are skipped.
#[must_use]
pub fn send_all(config: SmtpConfig, emails: Vec<Email>) -> Io<EmailError, ()> {
    Io::suspend(move || {
        let transport = build_transport(&config)?;
        emails.iter().try_for_each(|email| {
            transport.send(email.inner())
                .map(|_| ())
                .map_err(EmailError::from)
        })
    })
}

/// Create a `Resource` for an SMTP connection.
///
/// The transport is built on acquire and dropped on release.
/// Use this when you want explicit control over connection lifetime.
#[must_use]
pub fn smtp_resource(config: SmtpConfig) -> Resource<EmailError, SmtpHandle> {
    Resource::make(
        move || Io::suspend(move || {
            build_transport(&config).map(SmtpHandle)
        }),
        |_handle| Io::pure(()),
    )
}

/// A handle to an open SMTP connection.
///
/// Provides `send` for sending individual emails over the
/// already-established connection.
pub struct SmtpHandle(lettre::SmtpTransport);

impl SmtpHandle {
    /// Send an email over this connection.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Smtp` if the send fails.
    pub fn send(&self, email: &Email) -> Io<EmailError, ()> {
        // We need to call the transport synchronously here.
        // Since SmtpTransport::send takes &self, we can call it directly.
        self.0.send(email.inner())
            .map(|_response| Io::pure(()))
            .map_err(EmailError::from)
            .unwrap_or_else(|e| Io::suspend(move || Err(e)))
    }
}

fn build_transport(config: &SmtpConfig) -> Result<lettre::SmtpTransport, EmailError> {
    let builder = if config.starttls {
        lettre::SmtpTransport::starttls_relay(&config.host)
            .map_err(EmailError::from)?
    } else {
        lettre::SmtpTransport::relay(&config.host)
            .map_err(EmailError::from)?
    };

    let builder = config.port.into_iter()
        .fold(builder, lettre::transport::smtp::SmtpTransportBuilder::port);

    let builder = config.credentials.as_ref().into_iter()
        .fold(builder, |b, c| b.credentials(c.inner().clone()));

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smtp_config_builder() {
        let config = SmtpConfig::new("smtp.example.com")
            .port(465)
            .no_starttls();
        assert_eq!(config.host, "smtp.example.com");
        assert_eq!(config.port, Some(465));
        assert!(!config.starttls);
    }

    #[test]
    fn gmail_preset() {
        let config = SmtpConfig::gmail("user@gmail.com".into(), "app-pass".into());
        assert_eq!(config.host, "smtp.gmail.com");
        assert_eq!(config.port, Some(587));
        assert!(config.credentials.is_some());
    }

    #[test]
    fn outlook_preset() {
        let config = SmtpConfig::outlook("user@outlook.com".into(), "pass".into());
        assert_eq!(config.host, "smtp-mail.outlook.com");
        assert_eq!(config.port, Some(587));
        assert!(config.credentials.is_some());
    }

    #[test]
    fn credentials_construction() {
        let creds = Credentials::new("user".into(), "pass".into());
        // Just verify it doesn't panic; the inner type is opaque.
        let _ = creds.inner();
    }
}
