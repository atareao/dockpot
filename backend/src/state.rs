use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::config::Config;
use crate::db::Database;

// ───── Type aliases ────────────────────────────────────────────

pub type OidcStates = Arc<Mutex<HashMap<String, (String, std::time::Instant)>>>;
pub type CachedContainers = Arc<RwLock<Option<Vec<ContainerInfo>>>>;

// ───── OIDC State ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct OidcState {
    pub code_verifier: String,
    pub state: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ───── JWT Claims ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
}

// ───── OIDC Metadata ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct OidcMetadata {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub issuer: String,
    #[serde(rename = "jwks_uri")]
    pub jwks_uri: String,
}

// ───── Event types (SSE / broadcasting) ────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_tag: String,
    pub size_mb: f64,
    pub status: String,
    pub state: String,
    pub has_update: bool,
    pub updating: bool,
    pub compose_project: Option<String>,
    pub ports: Vec<String>,
    pub traefik_url: Option<String>,
    pub registry_url: String,
    #[serde(default)]
    pub last_check: Option<String>,
    #[serde(default)]
    pub next_check: Option<String>,
    #[serde(default)]
    pub last_remote_digest: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateEvent {
    pub containers: Vec<ContainerInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub container: String,
    pub status: String,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NotifEvent {
    pub container: String,
    pub status: String,
    pub timestamp: String,
}

// ───── Singleton HTTP client ───────────────────────────────────

pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create reqwest::Client")
    })
}

// ───── JWT Validator (Alloy-style) ────────────────────────────

/// JWT Validator que obtiene JWKS del issuer y valida tokens Bearer con RS256.
#[derive(Clone)]
pub struct JwtValidator {
    jwks: Arc<RwLock<Vec<jsonwebtoken::DecodingKey>>>,
    issuer: String,
    client_id: String,
}

impl JwtValidator {
    pub fn new(issuer: &str, client_id: &str) -> Self {
        Self {
            jwks: Arc::new(RwLock::new(Vec::new())),
            issuer: issuer.to_string(),
            client_id: client_id.to_string(),
        }
    }

    pub async fn fetch_jwks(&self) -> Result<(), String> {
        let jwks_url = format!(
            "{}/.well-known/jwks.json",
            self.issuer.trim_end_matches('/')
        );
        let client = http_client();
        let resp: serde_json::Value = client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| format!("failed to fetch JWKS: {e}"))?
            .json()
            .await
            .map_err(|e| format!("failed to parse JWKS response: {e}"))?;

        let keys = resp["keys"]
            .as_array()
            .ok_or_else(|| "JWKS response missing 'keys' array".to_string())?;

        let mut decoding_keys = Vec::new();
        for key in keys {
            if let (Some(n), Some(e)) = (
                key["n"].as_str().and_then(|s| {
                    base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, s)
                        .ok()
                }),
                key["e"].as_str().and_then(|s| {
                    base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, s)
                        .ok()
                }),
            ) {
                let dk = jsonwebtoken::DecodingKey::from_rsa_raw_components(&n, &e);
                decoding_keys.push(dk);
            }
        }

        tracing::info!(
            count = decoding_keys.len(),
            "JWKS fetched from {}",
            jwks_url
        );
        *self.jwks.write().await = decoding_keys;
        Ok(())
    }

    pub async fn validate_token(&self, token: &str) -> Result<JwtClaims, String> {
        let keys = {
            let jwks = self.jwks.read().await;
            if jwks.is_empty() {
                // Auto-fetch on first use
                drop(jwks);
                self.fetch_jwks().await?;
                return Box::pin(self.validate_token(token)).await;
            }
            jwks.clone()
        };

        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.client_id]);
        validation.validate_exp = true;

        for key in &keys {
            if let Ok(data) = jsonwebtoken::decode::<JwtClaims>(token, key, &validation) {
                return Ok(data.claims);
            }
        }
        Err("no matching JWK found for token".to_string())
    }
}

// ───── AppState ────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub docker: Docker,
    pub config: Config,
    pub db: Database,
    pub tx: broadcast::Sender<StateEvent>,
    pub update_tx: broadcast::Sender<UpdateProgress>,
    pub notif_tx: broadcast::Sender<NotifEvent>,
    pub oidc_states: OidcStates,
    pub oidc_metadata: Option<OidcMetadata>,
    pub jwt_validator: JwtValidator,
    pub cached_containers: CachedContainers,
    pub dev_mode: bool,
}

impl axum::extract::FromRef<AppState> for Docker {
    fn from_ref(state: &AppState) -> Self {
        state.docker.clone()
    }
}

impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl axum::extract::FromRef<AppState> for Database {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl axum::extract::FromRef<AppState> for OidcStates {
    fn from_ref(state: &AppState) -> Self {
        state.oidc_states.clone()
    }
}

impl axum::extract::FromRef<AppState> for Option<OidcMetadata> {
    fn from_ref(state: &AppState) -> Self {
        state.oidc_metadata.clone()
    }
}

impl axum::extract::FromRef<AppState> for JwtValidator {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_validator.clone()
    }
}

impl axum::extract::FromRef<AppState> for CachedContainers {
    fn from_ref(state: &AppState) -> Self {
        state.cached_containers.clone()
    }
}

impl axum::extract::FromRef<AppState> for broadcast::Sender<StateEvent> {
    fn from_ref(state: &AppState) -> Self {
        state.tx.clone()
    }
}

impl axum::extract::FromRef<AppState> for broadcast::Sender<UpdateProgress> {
    fn from_ref(state: &AppState) -> Self {
        state.update_tx.clone()
    }
}

impl axum::extract::FromRef<AppState> for broadcast::Sender<NotifEvent> {
    fn from_ref(state: &AppState) -> Self {
        state.notif_tx.clone()
    }
}

// ───── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── OidcMetadata deserialization ─────────────────────────

    #[test]
    fn test_oidc_metadata_deserialize_full() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "issuer": "https://auth.example.com",
            "jwks_uri": "https://auth.example.com/.well-known/jwks.json"
        }"#;
        let meta: OidcMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(
            meta.authorization_endpoint,
            "https://auth.example.com/authorize"
        );
        assert_eq!(meta.token_endpoint, "https://auth.example.com/token");
        assert_eq!(meta.userinfo_endpoint, "https://auth.example.com/userinfo");
        assert_eq!(meta.issuer, "https://auth.example.com");
        assert_eq!(
            meta.jwks_uri,
            "https://auth.example.com/.well-known/jwks.json"
        );
    }

    #[test]
    fn test_oidc_metadata_deserialize_minimal() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "issuer": "https://auth.example.com",
            "jwks_uri": "https://auth.example.com/.well-known/jwks.json"
        }"#;
        let meta: OidcMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.issuer, "https://auth.example.com");
    }

    #[test]
    fn test_oidc_metadata_jwks_uri_rename() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize",
            "token_endpoint": "https://auth.example.com/token",
            "userinfo_endpoint": "https://auth.example.com/userinfo",
            "issuer": "https://auth.example.com",
            "jwks_uri": "https://keys.example.com/jwks"
        }"#;
        let meta: OidcMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.jwks_uri, "https://keys.example.com/jwks");
    }

    #[test]
    fn test_oidc_metadata_rejects_missing_fields() {
        let json = r#"{
            "authorization_endpoint": "https://auth.example.com/authorize"
        }"#;
        let result: Result<OidcMetadata, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_oidc_metadata_rejects_empty_string() {
        let result: Result<OidcMetadata, _> = serde_json::from_str("");
        assert!(result.is_err());
    }

    // ── JwtValidator ─────────────────────────────────────────

    #[test]
    fn test_jwt_validator_new() {
        let validator = JwtValidator::new("https://issuer.test", "client-123");
        assert_eq!(validator.issuer, "https://issuer.test");
        assert_eq!(validator.client_id, "client-123");
    }

    #[test]
    fn test_jwt_validator_new_empty_jwks() {
        let validator = JwtValidator::new("https://issuer.test", "client-123");
        let jwks = validator.jwks.blocking_read();
        assert!(jwks.is_empty());
    }

    #[test]
    fn test_jwt_validator_clone() {
        let v1 = JwtValidator::new("https://issuer.test", "client-123");
        let v2 = v1.clone();
        assert_eq!(v2.issuer, "https://issuer.test");
        assert_eq!(v2.client_id, "client-123");
    }

    // ── http_client ──────────────────────────────────────────

    #[test]
    fn test_http_client_exists() {
        let client = http_client();
        assert_eq!(
            std::mem::size_of_val(client),
            std::mem::size_of::<reqwest::Client>()
        );
    }

    #[test]
    fn test_http_client_is_singleton() {
        let c1 = http_client();
        let c2 = http_client();
        assert!(std::ptr::eq(c1, c2));
    }

    // ── JwtClaims ────────────────────────────────────────────

    #[test]
    fn test_jwt_claims_serialize_roundtrip() {
        let claims = JwtClaims {
            sub: "user-123".into(),
            email: Some("user@example.com".into()),
            name: Some("Test User".into()),
            exp: 9999999999,
            iat: 1000000000,
            iss: "https://issuer.test".into(),
        };
        let json = serde_json::to_string(&claims).unwrap();
        let deserialized: JwtClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-123");
        assert_eq!(deserialized.email.unwrap(), "user@example.com");
        assert_eq!(deserialized.name.unwrap(), "Test User");
        assert_eq!(deserialized.exp, 9999999999);
        assert_eq!(deserialized.iss, "https://issuer.test");
    }

    #[test]
    fn test_jwt_claims_minimal() {
        let json = r#"{"sub":"u1","exp":9999,"iat":1000,"iss":"iss"}"#;
        let claims: JwtClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "u1");
        assert!(claims.email.is_none());
        assert!(claims.name.is_none());
    }

    // ── OidcState ────────────────────────────────────────────

    #[test]
    fn test_oidc_state_creation() {
        use chrono::Utc;
        let now = Utc::now();
        let state = OidcState {
            code_verifier: "abc123".into(),
            state: "xyz789".into(),
            created_at: now,
        };
        assert_eq!(state.code_verifier, "abc123");
        assert_eq!(state.state, "xyz789");
    }

    // ── ContainerInfo ────────────────────────────────────────

    #[test]
    fn test_container_info_serialize() {
        let info = ContainerInfo {
            id: "abc".into(),
            name: "test".into(),
            image: "nginx".into(),
            image_tag: "latest".into(),
            size_mb: 45.2,
            status: "running".into(),
            state: "running".into(),
            has_update: false,
            updating: false,
            compose_project: None,
            ports: vec!["80/tcp".into()],
            traefik_url: None,
            registry_url: "https://registry.example.com".into(),
            last_check: None,
            next_check: None,
            last_remote_digest: String::new(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"state\":\"running\""));
    }

    // ── StateEvent ───────────────────────────────────────────

    #[test]
    fn test_state_event_serialize() {
        let event = StateEvent { containers: vec![] };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, r#"{"containers":[]}"#);
    }

    // ── UpdateProgress ───────────────────────────────────────

    #[test]
    fn test_update_progress_serialize_roundtrip() {
        let progress = UpdateProgress {
            container: "web".into(),
            status: "pulling".into(),
            done: false,
            error: None,
        };
        let json = serde_json::to_string(&progress).unwrap();
        let deser: UpdateProgress = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.container, "web");
        assert_eq!(deser.status, "pulling");
        assert!(!deser.done);
        assert!(deser.error.is_none());
    }

    #[test]
    fn test_update_progress_with_error() {
        let progress = UpdateProgress {
            container: "db".into(),
            status: "failed".into(),
            done: true,
            error: Some("pull failed".into()),
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"error\":\"pull failed\""));
    }

    // ── NotifEvent ───────────────────────────────────────────

    #[test]
    fn test_notif_event_serialize() {
        let event = NotifEvent {
            container: "app".into(),
            status: "▶️ en ejecución".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"container\":\"app\""));
        assert!(json.contains("\"timestamp\":\"2026-01-01T00:00:00Z\""));
    }

    // ── OidcStates type ──────────────────────────────────────

    #[test]
    fn test_oidc_states_insert_and_get() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let states: OidcStates = Arc::new(Mutex::new(HashMap::new()));
            let key = "session-1".to_string();
            let val = ("code-verifier".to_string(), std::time::Instant::now());
            states.lock().await.insert(key.clone(), val.clone());
            let stored = states.lock().await.get(&key).cloned().unwrap();
            assert_eq!(stored.0, "code-verifier");
        });
    }

    // ── CachedContainers type ────────────────────────────────

    #[test]
    fn test_cached_containers_write_read() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let cache: CachedContainers = Arc::new(RwLock::new(None));
            assert!(cache.read().await.is_none());
            *cache.write().await = Some(vec![]);
            assert!(cache.read().await.is_some());
        });
    }

    // ── FromRef assertions (compile-time) ────────────────────

    /// Verify that every type used in AppState has a FromRef impl
    /// by creating an AppState and extracting each field via
    /// axum::extract::FromRef (simulated with a helper function).
    #[test]
    fn test_from_ref_docker() {
        fn extract(state: &AppState) -> Docker {
            axum::extract::FromRef::from_ref(state)
        }
        // Can't construct a real Docker, but the impl compiles
        let _ = extract;
    }

    #[test]
    fn test_from_ref_config() {
        fn extract(state: &AppState) -> Config {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_db() {
        fn extract(state: &AppState) -> Database {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_state_event_sender() {
        fn extract(state: &AppState) -> broadcast::Sender<StateEvent> {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_update_progress_sender() {
        fn extract(state: &AppState) -> broadcast::Sender<UpdateProgress> {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_notif_event_sender() {
        fn extract(state: &AppState) -> broadcast::Sender<NotifEvent> {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_oidc_states() {
        fn extract(state: &AppState) -> OidcStates {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_oidc_metadata() {
        fn extract(state: &AppState) -> Option<OidcMetadata> {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_jwt_validator() {
        fn extract(state: &AppState) -> JwtValidator {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }

    #[test]
    fn test_from_ref_cached_containers() {
        fn extract(state: &AppState) -> CachedContainers {
            axum::extract::FromRef::from_ref(state)
        }
        let _ = extract;
    }
}
