//! Message building: pure construction, no effects.
//!
//! Wraps lettre's `Message` builder with newtypes for addresses
//! and a simplified API.  Everything here is pure data; effects
//! only happen when you send via `transport`.

use crate::error::EmailError;

/// A validated email address (newtype over `lettre::Address`).
#[derive(Debug, Clone)]
pub struct Address(lettre::Address);

impl Address {
    /// Parse and validate an email address.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Address` if the string is not a valid email.
    pub fn parse(s: &str) -> Result<Self, EmailError> {
        s.parse::<lettre::Address>()
            .map(Self)
            .map_err(EmailError::from)
    }

    pub(crate) fn inner(&self) -> &lettre::Address {
        &self.0
    }
}

/// A named mailbox: "Display Name <email@example.com>".
#[derive(Debug, Clone)]
pub struct Mailbox(lettre::message::Mailbox);

impl Mailbox {
    /// Create a mailbox with a display name and address.
    #[must_use]
    pub fn new(name: Option<String>, address: &Address) -> Self {
        Self(lettre::message::Mailbox::new(name, address.inner().clone()))
    }

    /// Create a mailbox from just an address (no display name).
    #[must_use]
    pub fn from_address(address: &Address) -> Self {
        Self::new(None, address)
    }

    pub(crate) fn inner(&self) -> &lettre::message::Mailbox {
        &self.0
    }
}

/// An email message ready to send.
///
/// Built via `EmailBuilder`.  This is pure data; no effects.
#[derive(Debug, Clone)]
pub struct Email(lettre::Message);

impl Email {
    pub(crate) fn inner(&self) -> &lettre::Message {
        &self.0
    }
}

/// Builder for constructing an `Email`.
pub struct EmailBuilder {
    from: Option<Mailbox>,
    to: Vec<Mailbox>,
    subject: Option<String>,
    body: Option<String>,
    reply_to: Option<Mailbox>,
    cc: Vec<Mailbox>,
    bcc: Vec<Mailbox>,
}

impl EmailBuilder {
    /// Start building a new email.
    #[must_use]
    pub fn new() -> Self {
        Self {
            from: None,
            to: Vec::new(),
            subject: None,
            body: None,
            reply_to: None,
            cc: Vec::new(),
            bcc: Vec::new(),
        }
    }

    /// Set the sender.
    #[must_use]
    pub fn from(self, mailbox: Mailbox) -> Self {
        Self { from: Some(mailbox), ..self }
    }

    /// Add a recipient.
    #[must_use]
    pub fn to(self, mailbox: Mailbox) -> Self {
        Self {
            to: self.to.into_iter().chain(std::iter::once(mailbox)).collect(),
            ..self
        }
    }

    /// Set the subject line.
    #[must_use]
    pub fn subject(self, subject: impl Into<String>) -> Self {
        Self { subject: Some(subject.into()), ..self }
    }

    /// Set the plain text body.
    #[must_use]
    pub fn body(self, body: impl Into<String>) -> Self {
        Self { body: Some(body.into()), ..self }
    }

    /// Set the reply-to address.
    #[must_use]
    pub fn reply_to(self, mailbox: Mailbox) -> Self {
        Self { reply_to: Some(mailbox), ..self }
    }

    /// Add a CC recipient.
    #[must_use]
    pub fn cc(self, mailbox: Mailbox) -> Self {
        Self {
            cc: self.cc.into_iter().chain(std::iter::once(mailbox)).collect(),
            ..self
        }
    }

    /// Add a BCC recipient.
    #[must_use]
    pub fn bcc(self, mailbox: Mailbox) -> Self {
        Self {
            bcc: self.bcc.into_iter().chain(std::iter::once(mailbox)).collect(),
            ..self
        }
    }

    /// Build the email message.
    ///
    /// # Errors
    ///
    /// Returns `EmailError::Config` if required fields (from, to, subject, body)
    /// are missing, or `EmailError::Message` if lettre rejects the message.
    pub fn build(self) -> Result<Email, EmailError> {
        let from = self.from
            .ok_or_else(|| EmailError::Config { field: "from".into() })?;
        let first_to = self.to.first()
            .ok_or_else(|| EmailError::Config { field: "to".into() })?;
        let subject = self.subject
            .ok_or_else(|| EmailError::Config { field: "subject".into() })?;
        let body = self.body
            .ok_or_else(|| EmailError::Config { field: "body".into() })?;

        let builder = lettre::Message::builder()
            .from(from.inner().clone())
            .to(first_to.inner().clone())
            .subject(subject);

        // Add remaining To recipients
        let builder = self.to.iter().skip(1)
            .fold(builder, |b, m| b.to(m.inner().clone()));

        // Add CC recipients
        let builder = self.cc.iter()
            .fold(builder, |b, m| b.cc(m.inner().clone()));

        // Add BCC recipients
        let builder = self.bcc.iter()
            .fold(builder, |b, m| b.bcc(m.inner().clone()));

        // Add reply-to if set.  We fold over an Option-as-iterator
        // to avoid the double-move problem with map_or_else.
        let builder = self.reply_to.into_iter()
            .fold(builder, |b, m| b.reply_to(m.inner().clone()));

        builder.body(body)
            .map(Email)
            .map_err(EmailError::from)
    }
}

impl Default for EmailBuilder {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_parse_valid() -> Result<(), EmailError> {
        let addr = Address::parse("test@example.com")?;
        assert_eq!(addr.inner().to_string(), "test@example.com");
        Ok(())
    }

    #[test]
    fn address_parse_invalid() {
        assert!(Address::parse("not-an-email").is_err());
    }

    #[test]
    fn email_builder_requires_from() -> Result<(), EmailError> {
        let to = Address::parse("to@example.com")?;
        let result = EmailBuilder::new()
            .to(Mailbox::from_address(&to))
            .subject("test")
            .body("hello")
            .build();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn email_builder_requires_to() -> Result<(), EmailError> {
        let from = Address::parse("from@example.com")?;
        let result = EmailBuilder::new()
            .from(Mailbox::from_address(&from))
            .subject("test")
            .body("hello")
            .build();
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn email_builder_complete() -> Result<(), EmailError> {
        let from = Address::parse("from@example.com")?;
        let to = Address::parse("to@example.com")?;
        let _email = EmailBuilder::new()
            .from(Mailbox::from_address(&from))
            .to(Mailbox::from_address(&to))
            .subject("Hello")
            .body("World")
            .build()?;
        Ok(())
    }
}
