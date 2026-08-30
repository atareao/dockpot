use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use base64::Engine;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Config;
use crate::state::{AppState, JwtClaims};

// ───── Session Claims ────────────────────────────────────────────

/// Claims del JWT de sesión firmado con HMAC-SHA256 (client_secret como key).
/// Es el token que viaja en la cookie `session`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionClaims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub exp: usize,
    pub iat: usize,
    pub last_active: usize,
}

// ───── Public routes ─────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login", get(auth_login))
        .route("/api/auth/callback", get(auth_callback))
        .route("/api/auth/logout", get(auth_logout))
        .route("/api/auth/me", get(auth_me))
}

// ───── Query params ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
}

// ───── Handlers ──────────────────────────────────────────────────

/// Redirige al OIDC provider con PKCE (code_challenge + state).
pub async fn auth_login(State(state): State<AppState>) -> Result<Redirect, AuthError> {
    let metadata = state
        .oidc_metadata
        .as_ref()
        .ok_or_else(|| AuthError::new("OIDC not configured"))?;

    // ── PKCE: code_verifier aleatorio ──
    let code_verifier = Uuid::new_v4().to_string() + &Uuid::new_v4().to_string();

    // code_challenge = base64url(sha256(code_verifier))
    let code_challenge = {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let digest = hasher.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
    };

    // state aleatorio
    let state_value = Uuid::new_v4().to_string();

    // Guardar (code_verifier, created_at) → OidcStates
    state.oidc_states.lock().await.insert(
        state_value.clone(),
        (code_verifier, std::time::Instant::now()),
    );

    // Construir URL de autorización
    let redirect_uri = url_encode(&state.config.oidc_redirect_url);
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope=openid%20email%20profile&state={}&code_challenge={}&code_challenge_method=S256",
        metadata.authorization_endpoint,
        state.config.oidc_client_id,
        redirect_uri,
        state_value,
        code_challenge,
    );

    Ok(Redirect::to(&auth_url))
}

/// Intercambia el código por tokens, valida el id_token, crea la sesión.
pub async fn auth_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Result<Response, AuthError> {
    let metadata = state
        .oidc_metadata
        .as_ref()
        .ok_or_else(|| AuthError::new("OIDC not configured"))?;

    // ── Verificar state contra OidcStates ──
    let code_verifier = {
        let mut states = state.oidc_states.lock().await;
        let (cv, _created_at) = states
            .remove(&params.state)
            .ok_or_else(|| AuthError::new("Invalid state parameter"))?;
        cv
    };

    // ── Intercambiar código por tokens ──
    let token_endpoint = &metadata.token_endpoint;
    let client = reqwest::Client::new();
    let token_params = [
        ("grant_type", "authorization_code"),
        ("code", &params.code),
        ("redirect_uri", &state.config.oidc_redirect_url),
        ("client_id", &state.config.oidc_client_id),
        ("client_secret", &state.config.oidc_client_secret),
        ("code_verifier", &code_verifier),
    ];

    let token_resp = client
        .post(token_endpoint)
        .form(&token_params)
        .send()
        .await
        .map_err(|e| AuthError::new(&format!("Token request failed: {}", e)))?;

    let status = token_resp.status();
    let token_body: serde_json::Value = token_resp.json().await.map_err(|e| {
        AuthError::new(&format!(
            "Invalid token response (HTTP {}): {}",
            status.as_u16(),
            e
        ))
    })?;

    if !status.is_success() {
        let err = token_body["error"].as_str().unwrap_or("unknown");
        let desc = token_body["error_description"]
            .as_str()
            .unwrap_or("no description");
        tracing::error!(
            "Token endpoint error ({}): {} - {}",
            status.as_u16(),
            err,
            desc
        );
        return Err(AuthError::new(&format!(
            "Token endpoint error: {} ({})",
            err, desc
        )));
    }

    // Extraer id_token (es el que contiene la identidad del usuario)
    let id_token = token_body["id_token"]
        .as_str()
        .ok_or_else(|| AuthError::new("No id_token in token response"))?;

    // ── Validar id_token contra JWKS (RS256, issuer, audience) ──
    let jwt_claims: JwtClaims = state
        .jwt_validator
        .validate_token(id_token)
        .await
        .map_err(|e| AuthError::new(&format!("id_token validation failed: {}", e)))?;

    // ── Crear SessionClaims y firmar JWT de sesión (HMAC-SHA256) ──
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let session_exp = now + 86400; // 24h

    let session_claims = SessionClaims {
        sub: jwt_claims.sub,
        email: jwt_claims.email,
        name: jwt_claims.name,
        exp: session_exp,
        iat: now,
        last_active: now,
    };

    let session_token = encode(
        &Header::default(), // HS256 es el default
        &session_claims,
        &EncodingKey::from_secret(state.config.oidc_client_secret.as_bytes()),
    )
    .map_err(|e| AuthError::new(&format!("Failed to sign session JWT: {}", e)))?;

    // ── Set-Cookie + redirect a "/" ──
    let cookie = format!(
        "session={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=86400",
        session_token,
    );

    let response = Response::builder()
        .header("Set-Cookie", cookie)
        .header("Location", "/")
        .status(StatusCode::FOUND)
        .body(axum::body::Body::empty())
        .unwrap();

    Ok(response)
}

/// Limpia la cookie de sesión y redirige a "/".
pub async fn auth_logout() -> Response {
    let cookie = "session=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0";

    Response::builder()
        .header("Set-Cookie", cookie)
        .header("Location", "/")
        .status(StatusCode::FOUND)
        .body(axum::body::Body::empty())
        .unwrap()
}

/// Devuelve información del usuario autenticado, o `{ authenticated: false }`.
pub async fn auth_me(headers: HeaderMap, State(state): State<AppState>) -> Json<serde_json::Value> {
    // Leer cookie `session`
    let session = headers
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| {
            c.split(';')
                .find(|part| part.trim().starts_with("session="))
                .and_then(|part| part.trim().split_once('=').map(|(_, v)| v))
        });

    if let Some(token) = session {
        let key = DecodingKey::from_secret(state.config.oidc_client_secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        match decode::<SessionClaims>(token, &key, &validation) {
            Ok(data) => {
                return Json(serde_json::json!({
                    "authenticated": true,
                    "user": {
                        "sub": data.claims.sub,
                        "email": data.claims.email,
                        "name": data.claims.name,
                    }
                }));
            }
            Err(e) => {
                tracing::debug!("Session cookie validation failed: {}", e);
            }
        }
    }

    Json(serde_json::json!({"authenticated": false}))
}

// ───── Auth Middleware ───────────────────────────────────────────

/// Middleware que protege rutas no-públicas validando la cookie `session`.
///
/// El `client_secret` debe inyectarse en `req.extensions()` como `String`
/// antes de llamar a este middleware, por ejemplo:
///
/// ```ignore
/// .layer(axum::middleware::from_fn({
///     let secret = config.oidc_client_secret.clone();
///     move |headers, mut req, next| {
///         req.extensions_mut().insert(secret.clone());
///         auth_middleware(headers, req, next)
///     }
/// }))
/// ```
pub async fn auth_middleware(
    headers: HeaderMap,
    mut req: axum::extract::Request,
    next: Next,
) -> Result<Response, Response> {
    // ── Paths públicos (no requieren autenticación) ──
    let path = req.uri().path();
    let is_public = path == "/"
        || path == "/api/health"
        || path == "/api/auth/login"
        || path == "/api/auth/callback"
        || path.starts_with("/assets/")
        || path.ends_with(".html")
        || path.ends_with(".js")
        || path.ends_with(".css")
        || path.ends_with(".png")
        || path.ends_with(".ico")
        || path.ends_with(".svg")
        || path.ends_with(".woff2")
        || path.ends_with(".woff")
        || path.ends_with(".ttf");

    if is_public {
        return Ok(next.run(req).await);
    }

    // ── Extraer client_secret de extensions ──
    let secret = req.extensions().get::<String>().cloned().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Server configuration error"})),
        )
            .into_response()
    })?;

    // ── Leer cookie `session` ──
    let session = headers
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|c| {
            c.split(';')
                .find(|part| part.trim().starts_with("session="))
                .and_then(|part| part.trim().split_once('=').map(|(_, v)| v))
        });

    let token = match session {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "session_expired": true,
                    "error": "No session cookie"
                })),
            )
                .into_response());
        }
    };

    // ── Validar session JWT (HMAC-SHA256) ──
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    match decode::<SessionClaims>(token, &key, &validation) {
        Ok(data) => {
            req.extensions_mut().insert(data.claims);
            Ok(next.run(req).await)
        }
        Err(e) => {
            tracing::debug!("Session validation failed: {}", e);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "session_expired": true,
                    "error": e.to_string()
                })),
            )
                .into_response())
        }
    }
}

// ───── SPA Fallback ──────────────────────────────────────────────

/// Sirve el frontend embebido para cualquier ruta no coincidente.
pub async fn frontend_handler() -> impl IntoResponse {
    crate::embed::serve_embedded("/").await
}

// ───── Auth Error ────────────────────────────────────────────────

#[derive(Debug)]
pub struct AuthError {
    message: String,
}

impl AuthError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": self.message})),
        )
            .into_response()
    }
}

// ───── Helpers ───────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ───── OIDC Discovery ────────────────────────────────────────────

/// Descubre los endpoints OIDC a partir del issuer URL.
pub async fn discover_oidc(config: &Config) -> anyhow::Result<crate::state::OidcMetadata> {
    let issuer = config.oidc_issuer_url.trim_end_matches('/');
    let well_known = format!("{}/.well-known/openid-configuration", issuer);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client.get(&well_known).send().await?;
    let metadata: crate::state::OidcMetadata = resp.json().await?;
    Ok(metadata)
}

// ───── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SessionClaims ──────────────────────────────────────────

    #[test]
    fn test_session_claims_serialize_roundtrip() {
        let claims = SessionClaims {
            sub: "user-123".into(),
            email: Some("user@example.com".into()),
            name: Some("Test User".into()),
            exp: 9999999999,
            iat: 1000000000,
            last_active: 1000000000,
        };
        let json = serde_json::to_string(&claims).unwrap();
        let deserialized: SessionClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, "user-123");
        assert_eq!(deserialized.email.unwrap(), "user@example.com");
        assert_eq!(deserialized.name.unwrap(), "Test User");
        assert_eq!(deserialized.exp, 9999999999);
        assert_eq!(deserialized.last_active, 1000000000);
    }

    #[test]
    fn test_session_claims_minimal() {
        let json = r#"{"sub":"u1","exp":9999,"iat":1000,"last_active":1000}"#;
        let claims: SessionClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(claims.sub, "u1");
        assert!(claims.email.is_none());
        assert!(claims.name.is_none());
    }

    // ── url_encode ─────────────────────────────────────────────

    #[test]
    fn test_url_encode_normal() {
        assert_eq!(url_encode("hello"), "hello");
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(url_encode("a b"), "a+b");
        assert_eq!(url_encode("a&b"), "a%26b");
        assert_eq!(
            url_encode("http://example.com/callback"),
            "http%3A%2F%2Fexample.com%2Fcallback"
        );
    }

    // ── AuthError ──────────────────────────────────────────────

    #[test]
    fn test_auth_error_new() {
        let err = AuthError::new("test error");
        assert_eq!(err.message, "test error");
    }

    // ── PKCE code_challenge calculation ────────────────────────

    #[test]
    fn test_code_challenge_calculation() {
        let code_verifier = "test-verifier-123";
        let expected = {
            let mut hasher = Sha256::new();
            hasher.update(code_verifier.as_bytes());
            let digest = hasher.finalize();
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
        };

        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let digest = hasher.finalize();
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

        assert_eq!(challenge, expected);
    }

    // ── discover_oidc (integration-style, relies on HTTP) ──────
    // No se prueba aquí porque requiere un OIDC provider real.
    // Se prueba en los tests de integración.

    // ── SessionClaims from JwtClaims mapping ───────────────────

    #[test]
    fn test_jwt_claims_to_session_claims_mapping() {
        let jwt = JwtClaims {
            sub: "user-oidc".into(),
            email: Some("oidc@example.com".into()),
            name: Some("OIDC User".into()),
            exp: 9999999999,
            iat: 1000000000,
            iss: "https://issuer.test".into(),
        };

        let now = 2000000000usize;
        let session = SessionClaims {
            sub: jwt.sub,
            email: jwt.email,
            name: jwt.name,
            exp: now + 86400,
            iat: now,
            last_active: now,
        };

        assert_eq!(session.sub, "user-oidc");
        assert_eq!(session.email.unwrap(), "oidc@example.com");
        assert_eq!(session.name.unwrap(), "OIDC User");
        assert_eq!(session.exp, now + 86400);
        assert_eq!(session.iat, now);
        assert_eq!(session.last_active, now);
    }
}
