use std::fmt::{Display, Formatter};

const DEFAULT_GATEWAY_URL: &str = "wss://gateway.discord.gg/";
const DEFAULT_GATEWAY_VERSION: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayEncoding {
    Json,
}

impl GatewayEncoding {
    pub fn as_str(self) -> &'static str {
        match self {
            GatewayEncoding::Json => "json",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayCompression {
    ZlibStream,
}

impl GatewayCompression {
    pub fn as_str(self) -> &'static str {
        match self {
            GatewayCompression::ZlibStream => "zlib-stream",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatewayConnectionConfig {
    base_url: String,
    version: u8,
    encoding: GatewayEncoding,
    compression: Option<GatewayCompression>,
    shard: Option<(u32, u32)>,
}

impl Default for GatewayConnectionConfig {
    fn default() -> Self {
        Self::new(DEFAULT_GATEWAY_URL)
    }
}

impl GatewayConnectionConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            version: DEFAULT_GATEWAY_VERSION,
            encoding: GatewayEncoding::Json,
            compression: None,
            shard: None,
        }
    }

    pub fn version(mut self, version: u8) -> Self {
        self.version = version;
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn encoding(mut self, encoding: GatewayEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// Enable gateway zlib-stream compression.
    pub fn compression(mut self, compression: GatewayCompression) -> Self {
        self.compression = Some(compression);
        self
    }

    pub fn shard(mut self, shard_id: u32, total_shards: u32) -> Self {
        self.shard = Some((shard_id, total_shards));
        self
    }

    pub fn normalized_url(&self) -> String {
        let mut normalized = if self.base_url.contains("://") {
            self.base_url.clone()
        } else {
            format!("wss://{}", self.base_url.trim_start_matches('/'))
        };

        remove_query_param(&mut normalized, "compress");

        if !has_query_param(&normalized, "v") {
            append_query_param(&mut normalized, &format!("v={}", self.version));
        }

        if !has_query_param(&normalized, "encoding") {
            append_query_param(
                &mut normalized,
                &format!("encoding={}", self.encoding.as_str()),
            );
        }

        if let Some(compression) = self.compression {
            if !has_query_param(&normalized, "compress") {
                append_query_param(
                    &mut normalized,
                    &format!("compress={}", compression.as_str()),
                );
            }
        }

        if let Some((shard_id, total_shards)) = self.shard {
            if !has_query_param(&normalized, "shard") {
                append_query_param(&mut normalized, &format!("shard={shard_id},{total_shards}"));
            }
        }

        normalized
    }
}

impl Display for GatewayConnectionConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.normalized_url())
    }
}

fn has_query_param(url: &str, key: &str) -> bool {
    url.split_once('?')
        .map(|(_, query)| {
            query
                .split('&')
                .filter(|segment| !segment.is_empty())
                .any(|segment| segment.split('=').next() == Some(key))
        })
        .unwrap_or(false)
}

fn append_query_param(url: &mut String, param: &str) {
    if url.contains('?') {
        url.push('&');
    } else {
        url.push('?');
    }
    url.push_str(param);
}

fn remove_query_param(url: &mut String, key: &str) {
    let original = std::mem::take(url);
    let Some((base, query)) = original.split_once('?') else {
        *url = original;
        return;
    };

    let filtered_segments: Vec<&str> = query
        .split('&')
        .filter(|segment| !segment.is_empty())
        .filter(|segment| segment.split('=').next() != Some(key))
        .collect();

    *url = base.to_string();
    if !filtered_segments.is_empty() {
        url.push('?');
        url.push_str(&filtered_segments.join("&"));
    }
}

#[cfg(test)]
mod tests {
    use super::{GatewayCompression, GatewayConnectionConfig, GatewayEncoding};

    #[test]
    fn normalized_url_adds_default_query_values() {
        assert_eq!(
            GatewayConnectionConfig::default().normalized_url(),
            "wss://gateway.discord.gg/?v=10&encoding=json"
        );
    }

    #[test]
    fn normalized_url_includes_compression_when_configured_and_adds_shard() {
        let url = GatewayConnectionConfig::new(
            "wss://gateway.discord.gg/?encoding=json&compress=zlib-stream",
        )
        .compression(GatewayCompression::ZlibStream)
        .shard(2, 8)
        .normalized_url();

        assert_eq!(
            url,
            "wss://gateway.discord.gg/?encoding=json&v=10&compress=zlib-stream&shard=2,8"
        );
    }

    #[test]
    fn connection_config_supports_custom_base_url_version_encoding_and_display() {
        let config = GatewayConnectionConfig::new("gateway.discord.test")
            .with_base_url("/gateway.discord.test/socket")
            .version(11)
            .encoding(GatewayEncoding::Json);

        assert_eq!(GatewayEncoding::Json.as_str(), "json");
        assert_eq!(GatewayCompression::ZlibStream.as_str(), "zlib-stream");
        assert_eq!(
            config.normalized_url(),
            "wss://gateway.discord.test/socket?v=11&encoding=json"
        );
        assert_eq!(config.to_string(), config.normalized_url());
    }

    #[test]
    fn normalized_url_does_not_duplicate_existing_query_parameters() {
        let url =
            GatewayConnectionConfig::new("wss://gateway.discord.gg/?v=9&encoding=json&shard=1,4")
                .version(10)
                .shard(2, 8)
                .normalized_url();

        assert_eq!(url, "wss://gateway.discord.gg/?v=9&encoding=json&shard=1,4");
    }
}
