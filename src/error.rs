use std::fmt;

/// Wrapper around `reqwest::Error` that is `Clone`-able.
/// Stores the Display representation since reqwest errors are not Clone.
#[derive(Clone, Debug)]
pub struct HttpError {
    message: String,
}

impl HttpError {
    pub fn new(err: &reqwest::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HttpError {}

/// Unified error type for the discordrs framework.
///
/// Replaces the previous `Box<dyn Error>` with a matchable enum,
/// following the same pattern as discord.js's error hierarchy.
#[derive(Clone, Debug)]
pub enum DiscordError {
    /// Discord API returned an error response (4xx/5xx).
    Api {
        status: u16,
        code: Option<u64>,
        message: String,
    },
    /// An HTTP transport error occurred.
    Http(HttpError),
    /// A rate limit was encountered.
    RateLimit { route: String, retry_after: f64 },
    /// JSON serialization or deserialization failed.
    Json(String),
    /// An I/O error occurred.
    Io(String),
    /// A model validation or data error.
    Model { message: String },
    /// A gateway protocol error.
    Gateway { message: String },
    /// A voice subsystem error.
    Voice { message: String },
    /// A cache operation error.
    Cache { message: String },
}

impl DiscordError {
    pub fn api(status: u16, code: Option<u64>, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn model(message: impl Into<String>) -> Self {
        Self::Model {
            message: message.into(),
        }
    }

    pub fn gateway(message: impl Into<String>) -> Self {
        Self::Gateway {
            message: message.into(),
        }
    }

    pub fn voice(message: impl Into<String>) -> Self {
        Self::Voice {
            message: message.into(),
        }
    }

    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache {
            message: message.into(),
        }
    }

    /// Returns the HTTP status code if this is an API error.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Returns the Discord error code if this is an API error.
    pub fn discord_code(&self) -> Option<u64> {
        match self {
            Self::Api { code, .. } => *code,
            _ => None,
        }
    }
}

impl fmt::Display for DiscordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api {
                status,
                code,
                message,
            } => {
                write!(f, "Discord API error (status {status}")?;
                if let Some(code) = code {
                    write!(f, ", code {code}")?;
                }
                write!(f, "): {message}")
            }
            Self::Http(err) => write!(f, "HTTP error: {err}"),
            Self::RateLimit { route, retry_after } => write!(
                f,
                "Rate limited on route '{route}', retry after {retry_after}s"
            ),
            Self::Json(msg) => write!(f, "JSON error: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Model { message } => write!(f, "Model error: {message}"),
            Self::Gateway { message } => write!(f, "Gateway error: {message}"),
            Self::Voice { message } => write!(f, "Voice error: {message}"),
            Self::Cache { message } => write!(f, "Cache error: {message}"),
        }
    }
}

impl std::error::Error for DiscordError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for DiscordError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(HttpError::new(&err))
    }
}

impl From<serde_json::Error> for DiscordError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl From<std::io::Error> for DiscordError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<std::env::VarError> for DiscordError {
    fn from(err: std::env::VarError) -> Self {
        Self::Model {
            message: err.to_string(),
        }
    }
}

impl From<String> for DiscordError {
    fn from(message: String) -> Self {
        Self::Model { message }
    }
}

impl From<&str> for DiscordError {
    fn from(message: &str) -> Self {
        Self::Model {
            message: message.to_string(),
        }
    }
}

#[cfg(feature = "interactions")]
impl From<hex::FromHexError> for DiscordError {
    fn from(err: hex::FromHexError) -> Self {
        Self::Model {
            message: err.to_string(),
        }
    }
}

#[cfg(any(feature = "gateway", feature = "voice"))]
impl From<tokio_tungstenite::tungstenite::Error> for DiscordError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Gateway {
            message: err.to_string(),
        }
    }
}

#[cfg(feature = "interactions")]
impl From<ed25519_dalek::SignatureError> for DiscordError {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        Self::Model {
            message: err.to_string(),
        }
    }
}

/// Backward-compatible type alias. Deprecated in favor of [`DiscordError`].
#[deprecated(since = "0.4.0", note = "Use DiscordError instead")]
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
