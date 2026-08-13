//! Shared HTTP client used by catalog, download, and Hex.pm calls.

use once_cell::sync::Lazy;
use reqwest::Client;
use std::time::Duration;

/// GitHub requires a descriptive UA or it may reject the call.
pub fn user_agent() -> String {
    format!(
        "Elin/{} (+https://github.com/belurgas/elin)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Process-wide client with rustls. Request-level timeouts override the default.
pub static HTTP: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent(user_agent())
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(45))
        .build()
        .expect("failed to build HTTP client")
});
