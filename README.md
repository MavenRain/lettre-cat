# lettre-cat

Email sending built on [comp-cat-rs](https://crates.io/crates/comp-cat-rs).  Wraps [lettre](https://crates.io/crates/lettre) with lazy, composable effects.  All sending returns `Io<EmailError, ()>`.  Nothing happens until `.run()`.

## Installation

```toml
[dependencies]
lettre-cat = "0.1"
```

## Quick start

```rust
use lettre_cat::message::{EmailBuilder, Address, Mailbox};
use lettre_cat::transport::{SmtpConfig, Credentials, send};

let from = Address::parse("me@example.com")?;
let to = Address::parse("you@example.com")?;

let email = EmailBuilder::new()
    .from(Mailbox::from_address(&from))
    .to(Mailbox::from_address(&to))
    .subject("Hello from lettre-cat")
    .body("Sent via comp-cat-rs effects!")
    .build()?;

let config = SmtpConfig::gmail(
    "me@gmail.com".into(),
    "app-password".into(),
);

// Nothing sends until .run()
send(config, email).run()?;
```

## Building messages

Messages are pure data.  No effects, no side effects, no network calls.  Build them freely and pass them around.

```rust
use lettre_cat::message::{EmailBuilder, Address, Mailbox};

let from = Address::parse("sender@example.com")?;
let to = Address::parse("recipient@example.com")?;
let cc = Address::parse("cc@example.com")?;

let email = EmailBuilder::new()
    .from(Mailbox::new(Some("Sender Name".into()), &from))
    .to(Mailbox::from_address(&to))
    .cc(Mailbox::from_address(&cc))
    .reply_to(Mailbox::from_address(&from))
    .subject("Meeting tomorrow")
    .body("See you at 10am.")
    .build()?;
```

**Builder methods:**

| Method | Description |
|--------|-------------|
| `from(Mailbox)` | Set the sender (required) |
| `to(Mailbox)` | Add a recipient (at least one required) |
| `cc(Mailbox)` | Add a CC recipient |
| `bcc(Mailbox)` | Add a BCC recipient |
| `reply_to(Mailbox)` | Set the reply-to address |
| `subject(impl Into<String>)` | Set the subject line (required) |
| `body(impl Into<String>)` | Set the plain text body (required) |

All methods are chainable.  `build()` validates that `from`, `to`, `subject`, and `body` are present.

## SMTP transport

### Single email

```rust
use lettre_cat::transport::{SmtpConfig, send};

send(config, email).run()?;
```

### Batch sending

Opens one SMTP connection, sends all emails, then closes:

```rust
use lettre_cat::transport::{SmtpConfig, send_all};

let emails = vec![email_a, email_b, email_c];
send_all(config, emails).run()?;
```

### Connection resource

For explicit control over the SMTP connection lifecycle:

```rust
use lettre_cat::transport::{SmtpConfig, smtp_resource};

let resource = smtp_resource(config);
let result = resource.use_resource(|handle| {
    handle.send(&email_a)
        .flat_map(|()| handle.send(&email_b))
}).run()?;
```

The connection is opened on acquire and dropped on release, even if sending fails.

### Configuration

```rust
use lettre_cat::transport::{SmtpConfig, Credentials};

// Manual configuration
let config = SmtpConfig::new("smtp.example.com")
    .port(465)
    .credentials(Credentials::new("user".into(), "pass".into()))
    .no_starttls();  // use implicit TLS instead of STARTTLS

// Presets
let gmail = SmtpConfig::gmail("user@gmail.com".into(), "app-password".into());
let outlook = SmtpConfig::outlook("user@outlook.com".into(), "password".into());
```

| Method | Default | Description |
|--------|---------|-------------|
| `port(u16)` | 587 (STARTTLS) / 465 (implicit TLS) | SMTP port |
| `credentials(Credentials)` | None | SMTP authentication |
| `no_starttls()` | STARTTLS enabled | Use implicit TLS instead |

## Error handling

`EmailError` covers all failure modes:

| Variant | Source | When |
|---------|--------|------|
| `Smtp(lettre::transport::smtp::Error)` | lettre | Connection failed, auth failed, send rejected |
| `Message(lettre::error::Error)` | lettre | Invalid message construction |
| `Address(lettre::address::AddressError)` | lettre | Malformed email address |
| `Config { field }` | lettre-cat | Missing required builder field |

All variants implement `From` for `?` ergonomics.

## Composing with comp-cat-rs

### Error recovery

```rust
let result = send(config, email)
    .handle_error(|e| {
        eprintln!("Send failed: {e}");
    });
```

### Retry on failure

```rust
let attempt_1 = send(config.clone(), email.clone());
let attempt_2 = send(config, email);

let with_retry = attempt_1
    .handle_error_with(|_| attempt_2);
```

### Concurrent sending via Fiber

```rust
use comp_cat_rs::effect::fiber::par_zip;

let send_a = send(config_a, email_a);
let send_b = send(config_b, email_b);

// Both send concurrently on separate threads
par_zip(
    send_a.map_error(|e| /* ... */),
    send_b.map_error(|e| /* ... */),
).run()?;
```

## Why not just lettre?

lettre is excellent.  lettre-cat adds:

- **Lazy evaluation**: `send()` returns an `Io`, not a `Result`.  Nothing executes until `.run()`.  You can build, compose, and transform send pipelines before committing.
- **Resource safety**: `smtp_resource` uses the bracket pattern to guarantee the SMTP connection is closed, even on error.
- **Composability**: chain email sending with CSV processing (csv-cat), LLM calls (rig-cat), or any other `Io`-based effect via `flat_map`.
- **Concurrency**: `Fiber::fork` and `par_zip` for parallel sends, with no async/tokio.

## License

MIT
