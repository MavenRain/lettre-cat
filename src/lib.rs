//! # lettre-cat
//!
//! Email sending built on [`comp-cat-rs`](https://crates.io/crates/comp-cat-rs).
//!
//! Wraps [`lettre`](https://crates.io/crates/lettre) with lazy, composable effects.
//! All sending returns `Io<EmailError, ()>`.  SMTP connections are managed
//! via `Resource`.  Nothing happens until `.run()`.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use lettre_cat::message::{EmailBuilder, Address, Mailbox};
//! use lettre_cat::transport::{SmtpConfig, Credentials, send};
//!
//! let email = EmailBuilder::new()
//!     .from(Mailbox::from_address(Address::parse("me@example.com")?))
//!     .to(Mailbox::from_address(Address::parse("you@example.com")?))
//!     .subject("Hello from lettre-cat")
//!     .body("Sent via comp-cat-rs effects!")
//!     .build()?;
//!
//! let config = SmtpConfig::gmail("me@gmail.com".into(), "app-password".into());
//!
//! // Nothing sends until .run()
//! send(config, email).run()?;
//! ```

pub mod error;
pub mod message;
pub mod transport;
