//! Shared HTTP client used by catalog, download, and Hex.pm calls.

use once_cell::sync::Lazy;
use reqwest::Client;
use std::time::Duration;

/// Default User-Agent. GitHub requires a descriptive UA or it may reject the call.
pub const USER_AGENT: &str = "Elin/0.1.0 (+https://github.com/elin-app/elin)";

/// Process-wide client with rustls. Request-level timeouts override the default.
pub static HTTP: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(45))
        .build()
        .expect("failed to build HTTP client")
});
