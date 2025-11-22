use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use chrono::{DateTime, Utc};
use crate::storage::config::Config;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Failed to read token file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse token: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Token has expired")]
    TokenExpired,
    #[error("No refresh token available")]
    NoRefreshToken,
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("OAuth error: {0}")]
    OAuthError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfo {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub token_type: String,
}

pub struct TokenStorage {
    path: PathBuf,
}

impl TokenStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn save_token(&self, token: &TokenInfo) -> Result<(), AuthError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(token)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn load_token(&self) -> Result<TokenInfo, AuthError> {
        let content = std::fs::read_to_string(&self.path)?;
        let token: TokenInfo = serde_json::from_str(&content)?;
        Ok(token)
    }

    pub fn is_expired(&self, token: &TokenInfo) -> bool {
        token.expires_at <= Utc::now()
    }

    pub fn needs_refresh(&self, token: &TokenInfo) -> bool {
        let buffer = chrono::Duration::minutes(5);
        token.expires_at <= Utc::now() + buffer
    }
}

impl TokenInfo {
    pub fn new(access_token: String, expires_in_seconds: i64) -> Self {
        Self {
            access_token,
            refresh_token: None,
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in_seconds),
            token_type: "Bearer".to_string(),
        }
    }

    pub fn with_refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.expires_at > Utc::now()
    }
}

pub struct GoogleAuthenticator {
    config: Config,
    storage: TokenStorage,
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: String,
}

impl GoogleAuthenticator {
    pub fn new(config: Config) -> Self {
        let token_path = config.google.token_cache.clone();
        let storage = TokenStorage::new(token_path);
        let client = reqwest::Client::new();

        Self {
            config,
            storage,
            client,
        }
    }

    pub async fn get_valid_token(&mut self) -> Result<TokenInfo, AuthError> {
        match self.storage.load_token() {
            Ok(token) if token.is_valid() => Ok(token),
            Ok(token) if self.storage.needs_refresh(&token) => {
                self.refresh_token(&token).await
            }
            _ => Err(AuthError::TokenExpired),
        }
    }

    pub async fn refresh_token(&mut self, token: &TokenInfo) -> Result<TokenInfo, AuthError> {
        let refresh_token = token.refresh_token.as_ref()
            .ok_or(AuthError::NoRefreshToken)?;

        let params = [
            ("client_id", self.config.google.client_id.as_str()),
            ("client_secret", self.config.google.client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::OAuthError(error_text));
        }

        let token_response: TokenResponse = response.json().await?;

        let new_token = TokenInfo::new(token_response.access_token, token_response.expires_in)
            .with_refresh_token(refresh_token.clone());

        self.storage.save_token(&new_token)?;

        Ok(new_token)
    }

    pub fn get_auth_url(&self) -> String {
        let redirect_uri = "http://localhost:8080";
        let scope = "https://www.googleapis.com/auth/calendar";

        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            urlencoding::encode(&self.config.google.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(scope)
        )
    }

    pub async fn exchange_code_for_token(&mut self, code: &str) -> Result<TokenInfo, AuthError> {
        let params = [
            ("client_id", self.config.google.client_id.as_str()),
            ("client_secret", self.config.google.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", "http://localhost:8080"),
            ("grant_type", "authorization_code"),
        ];

        let response = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AuthError::OAuthError(error_text));
        }

        let token_response: TokenResponse = response.json().await?;

        let new_token = TokenInfo::new(token_response.access_token, token_response.expires_in)
            .with_refresh_token(
                token_response.refresh_token
                    .ok_or(AuthError::NoRefreshToken)?
            );

        self.storage.save_token(&new_token)?;

        Ok(new_token)
    }

    pub fn print_auth_instructions(&self) {
        println!("\n=== Google Calendar Authentication ===\n");
        println!("To authenticate with Google Calendar:");
        println!("1. Visit this URL in your browser:\n");
        println!("{}\n", self.get_auth_url());
        println!("2. Sign in and authorize the application");
        println!("3. After authorizing, you'll be redirected to localhost:8080");
        println!("4. Copy the 'code' parameter from the URL");
        println!("5. Paste it when prompted\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_token() -> TokenInfo {
        TokenInfo::new("test_access_token".to_string(), 3600)
    }

    fn create_expired_token() -> TokenInfo {
        TokenInfo {
            access_token: "expired_token".to_string(),
            refresh_token: Some("refresh_token".to_string()),
            expires_at: Utc::now() - chrono::Duration::hours(1),
            token_type: "Bearer".to_string(),
        }
    }

    #[test]
    fn new_token_is_valid() {
        let token = create_test_token();
        assert!(token.is_valid());
    }

    #[test]
    fn expired_token_is_not_valid() {
        let token = create_expired_token();
        assert!(!token.is_valid());
    }

    #[test]
    fn token_with_refresh_token() {
        let token = create_test_token()
            .with_refresh_token("refresh_token".to_string());

        assert_eq!(token.refresh_token, Some("refresh_token".to_string()));
    }

    #[test]
    fn save_token_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("token.json");
        let storage = TokenStorage::new(token_path.clone());
        let token = create_test_token();

        storage.save_token(&token).unwrap();

        assert!(token_path.exists());
    }

    #[test]
    fn load_token_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("token.json");
        let storage = TokenStorage::new(token_path.clone());
        let original_token = create_test_token()
            .with_refresh_token("refresh".to_string());

        storage.save_token(&original_token).unwrap();
        let loaded_token = storage.load_token().unwrap();

        assert_eq!(loaded_token.access_token, original_token.access_token);
        assert_eq!(loaded_token.refresh_token, original_token.refresh_token);
    }

    #[test]
    fn load_nonexistent_token_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let token_path = temp_dir.path().join("nonexistent.json");
        let storage = TokenStorage::new(token_path);

        let result = storage.load_token();

        assert!(result.is_err());
    }

    #[test]
    fn is_expired_detects_expired_token() {
        let storage = TokenStorage::new(PathBuf::from("/tmp/token.json"));
        let token = create_expired_token();

        assert!(storage.is_expired(&token));
    }

    #[test]
    fn is_expired_returns_false_for_valid_token() {
        let storage = TokenStorage::new(PathBuf::from("/tmp/token.json"));
        let token = create_test_token();

        assert!(!storage.is_expired(&token));
    }

    #[test]
    fn needs_refresh_detects_soon_to_expire_token() {
        let storage = TokenStorage::new(PathBuf::from("/tmp/token.json"));
        let token = TokenInfo {
            access_token: "token".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: Utc::now() + chrono::Duration::minutes(3),
            token_type: "Bearer".to_string(),
        };

        assert!(storage.needs_refresh(&token));
    }

    #[test]
    fn needs_refresh_returns_false_for_fresh_token() {
        let storage = TokenStorage::new(PathBuf::from("/tmp/token.json"));
        let token = create_test_token();

        assert!(!storage.needs_refresh(&token));
    }
}
