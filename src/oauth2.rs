use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::DiscordError;
use crate::types::invalid_data_error;

const API_BASE: &str = "https://discord.com/api/v10";
const AUTHORIZE_BASE: &str = "https://discord.com/oauth2/authorize";

#[derive(Clone, Debug)]
pub struct OAuth2Client {
    client: Client,
    client_id: String,
    client_secret: Option<String>,
    #[cfg(test)]
    api_base: String,
    #[cfg(test)]
    authorize_base: String,
}

impl OAuth2Client {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            client_id: client_id.into(),
            client_secret: Some(client_secret.into()),
            #[cfg(test)]
            api_base: API_BASE.to_string(),
            #[cfg(test)]
            authorize_base: AUTHORIZE_BASE.to_string(),
        }
    }

    pub fn public_client(client_id: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            client_id: client_id.into(),
            client_secret: None,
            #[cfg(test)]
            api_base: API_BASE.to_string(),
            #[cfg(test)]
            authorize_base: AUTHORIZE_BASE.to_string(),
        }
    }

    #[cfg(test)]
    fn new_with_base_url(
        client_id: impl Into<String>,
        client_secret: Option<String>,
        api_base: impl Into<String>,
        authorize_base: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            client_id: client_id.into(),
            client_secret,
            api_base: api_base.into(),
            authorize_base: authorize_base.into(),
        }
    }

    pub fn authorization_url(
        &self,
        request: OAuth2AuthorizationRequest,
    ) -> Result<String, DiscordError> {
        request.validate()?;
        let scopes = request
            .scopes
            .iter()
            .map(OAuth2Scope::as_str)
            .collect::<Vec<_>>()
            .join(" ");

        let mut pairs = vec![
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", request.redirect_uri.as_str()),
            ("response_type", request.response_type.as_str()),
            ("scope", scopes.as_str()),
        ];
        if let Some(state) = request.state.as_deref() {
            pairs.push(("state", state));
        }
        if let Some(prompt) = request.prompt.as_deref() {
            pairs.push(("prompt", prompt));
        }
        let mut url = self.authorize_base().to_string();
        url.push('?');
        url.push_str(&encode_form_pairs(&pairs));
        if let Some(integration_type) = request.integration_type {
            url.push('&');
            url.push_str("integration_type=");
            url.push_str(&integration_type.to_string());
        }
        Ok(url)
    }

    pub async fn exchange_code(
        &self,
        request: OAuth2CodeExchange,
    ) -> Result<OAuth2TokenResponse, DiscordError> {
        request.validate()?;
        let mut form = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", request.code),
            ("redirect_uri", request.redirect_uri),
            ("client_id", self.client_id.clone()),
        ];
        if let Some(client_secret) = &self.client_secret {
            form.push(("client_secret", client_secret.clone()));
        }
        self.send_token_request(&form).await
    }

    pub async fn refresh_token(
        &self,
        request: OAuth2RefreshToken,
    ) -> Result<OAuth2TokenResponse, DiscordError> {
        request.validate()?;
        let mut form = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", request.refresh_token),
            ("client_id", self.client_id.clone()),
        ];
        if let Some(client_secret) = &self.client_secret {
            form.push(("client_secret", client_secret.clone()));
        }
        self.send_token_request(&form).await
    }

    async fn send_token_request(
        &self,
        form: &[(&str, String)],
    ) -> Result<OAuth2TokenResponse, DiscordError> {
        let response = self
            .client
            .post(format!("{}/oauth2/token", self.api_base()))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(encode_owned_form_pairs(form))
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Discord OAuth2 token request failed ({status}): {body}").into());
        }
        Ok(response.json::<OAuth2TokenResponse>().await?)
    }

    #[cfg(test)]
    fn api_base(&self) -> &str {
        &self.api_base
    }

    #[cfg(not(test))]
    fn api_base(&self) -> &str {
        API_BASE
    }

    #[cfg(test)]
    fn authorize_base(&self) -> &str {
        &self.authorize_base
    }

    #[cfg(not(test))]
    fn authorize_base(&self) -> &str {
        AUTHORIZE_BASE
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuth2AuthorizationRequest {
    pub redirect_uri: String,
    pub scopes: Vec<OAuth2Scope>,
    pub response_type: String,
    pub state: Option<String>,
    pub prompt: Option<String>,
    pub integration_type: Option<u8>,
}

impl OAuth2AuthorizationRequest {
    pub fn code(
        redirect_uri: impl Into<String>,
        scopes: impl IntoIterator<Item = OAuth2Scope>,
    ) -> Self {
        Self {
            redirect_uri: redirect_uri.into(),
            scopes: scopes.into_iter().collect(),
            response_type: "code".to_string(),
            state: None,
            prompt: None,
            integration_type: None,
        }
    }

    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn integration_type(mut self, integration_type: u8) -> Self {
        self.integration_type = Some(integration_type);
        self
    }

    fn validate(&self) -> Result<(), DiscordError> {
        if self.redirect_uri.trim().is_empty() {
            return Err(invalid_data_error("redirect_uri must not be empty"));
        }
        if self.scopes.is_empty() {
            return Err(invalid_data_error("at least one OAuth2 scope is required"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct OAuth2Scope(String);

impl OAuth2Scope {
    pub fn new(scope: impl Into<String>) -> Self {
        Self(scope.into())
    }

    pub fn identify() -> Self {
        Self::new("identify")
    }

    pub fn email() -> Self {
        Self::new("email")
    }

    pub fn guilds() -> Self {
        Self::new("guilds")
    }

    pub fn guilds_join() -> Self {
        Self::new("guilds.join")
    }

    pub fn applications_commands_update() -> Self {
        Self::new("applications.commands.update")
    }

    pub fn role_connections_write() -> Self {
        Self::new("role_connections.write")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuth2CodeExchange {
    pub code: String,
    pub redirect_uri: String,
}

impl OAuth2CodeExchange {
    pub fn new(code: impl Into<String>, redirect_uri: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            redirect_uri: redirect_uri.into(),
        }
    }

    fn validate(&self) -> Result<(), DiscordError> {
        if self.code.trim().is_empty() {
            return Err(invalid_data_error("code must not be empty"));
        }
        if self.redirect_uri.trim().is_empty() {
            return Err(invalid_data_error("redirect_uri must not be empty"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuth2RefreshToken {
    pub refresh_token: String,
}

impl OAuth2RefreshToken {
    pub fn new(refresh_token: impl Into<String>) -> Self {
        Self {
            refresh_token: refresh_token.into(),
        }
    }

    fn validate(&self) -> Result<(), DiscordError> {
        if self.refresh_token.trim().is_empty() {
            return Err(invalid_data_error("refresh_token must not be empty"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

fn encode_form_pairs(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_owned_form_pairs(pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{
        OAuth2AuthorizationRequest, OAuth2Client, OAuth2CodeExchange, OAuth2RefreshToken,
        OAuth2Scope,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn authorization_url_encodes_scope_state_and_prompt() {
        let client = OAuth2Client::new_with_base_url(
            "123",
            Some("secret".to_string()),
            "http://127.0.0.1/api",
            "https://discord.com/oauth2/authorize",
        );
        let url = client
            .authorization_url(
                OAuth2AuthorizationRequest::code(
                    "https://app.example/callback",
                    [OAuth2Scope::identify(), OAuth2Scope::guilds_join()],
                )
                .state("state with space")
                .prompt("consent")
                .integration_type(1),
            )
            .unwrap();

        assert!(url.contains("client_id=123"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp.example%2Fcallback"));
        assert!(url.contains("scope=identify+guilds.join"));
        assert!(url.contains("state=state+with+space"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("integration_type=1"));
    }

    #[test]
    fn authorization_url_rejects_missing_scope() {
        let client = OAuth2Client::public_client("123");
        let error = client
            .authorization_url(OAuth2AuthorizationRequest::code(
                "https://app.example/callback",
                [],
            ))
            .unwrap_err();

        assert!(error.to_string().contains("at least one OAuth2 scope"));
    }

    #[tokio::test]
    async fn token_exchange_sends_form_without_bot_authorization() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0_u8; 4096];
            let received = stream.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..received]);

            assert!(request.starts_with("POST /oauth2/token HTTP/1.1"));
            assert!(request.contains("content-type: application/x-www-form-urlencoded"));
            assert!(!request.to_ascii_lowercase().contains("authorization: bot"));
            assert!(request.contains(
                "grant_type=authorization_code&code=abc&redirect_uri=https%3A%2F%2Fapp.example%2Fcallback&client_id=123&client_secret=secret"
            ));

            let body = r#"{"access_token":"access","token_type":"Bearer","expires_in":3600,"refresh_token":"refresh","scope":"identify"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let client = OAuth2Client::new_with_base_url(
            "123",
            Some("secret".to_string()),
            format!("http://127.0.0.1:{port}"),
            "https://discord.com/oauth2/authorize",
        );
        let token = client
            .exchange_code(OAuth2CodeExchange::new(
                "abc",
                "https://app.example/callback",
            ))
            .await
            .unwrap();

        assert_eq!(token.access_token, "access");
        assert_eq!(token.refresh_token.as_deref(), Some("refresh"));
        server.await.unwrap();
    }

    #[test]
    fn token_requests_validate_required_fields() {
        assert!(OAuth2CodeExchange::new("", "https://app.example/callback")
            .validate()
            .unwrap_err()
            .to_string()
            .contains("code must not be empty"));
        assert!(OAuth2RefreshToken::new("")
            .validate()
            .unwrap_err()
            .to_string()
            .contains("refresh_token must not be empty"));
    }
}
