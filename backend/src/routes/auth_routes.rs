use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::{AppState, Claims, OidcState};

#[derive(Deserialize)]
pub struct LoginQuery {
    pub redirect: Option<String>,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, String> {
    let metadata = state
        .oidc_metadata
        .as_ref()
        .ok_or("OIDC not configured".to_string())?;

    // Generate PKCE challenge
    let code_verifier = Uuid::new_v4().to_string() + &Uuid::new_v4().to_string();
    let code_challenge = {
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let result = hasher.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(result)
    };

    let state_value = Uuid::new_v4().to_string();
    let redirect_uri = query.redirect.unwrap_or_default();

    let oidc_state = OidcState {
        code_verifier,
        state: state_value.clone(),
        created_at: chrono::Utc::now(),
    };

    state
        .oidc_states
        .lock()
        .await
        .insert(state_value.clone(), oidc_state);

    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope=openid%20email%20profile&state={}&code_challenge={}&code_challenge_method=S256",
        metadata.authorization_endpoint,
        state.config.oidc_client_id,
        urlencoding(&state.config.oidc_redirect_url),
        state_value,
        code_challenge,
    );

    let redirect = Redirect::to(&auth_url);

    if !redirect_uri.is_empty() {
        state
            .oidc_states
            .lock()
            .await
            .entry(state_value)
            .and_modify(|s| s.state = format!("{}:{}", s.state, redirect_uri));
    }

    Ok(redirect)
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response, String> {
    let metadata = state.oidc_metadata.as_ref().ok_or("OIDC not configured")?;

    // Check if OIDC provider returned an error
    if let Some(error) = params.get("error") {
        let desc = params
            .get("error_description")
            .map(|s| s.as_str())
            .unwrap_or("Unknown error");
        tracing::error!("OIDC callback error: {} - {}", error, desc);
        return Err(format!(
            "OIDC provider returned an error: {} ({})",
            error, desc
        ));
    }

    let code = params
        .get("code")
        .ok_or_else(|| "Missing authorization code in callback".to_string())?;
    let state_param = params
        .get("state")
        .ok_or_else(|| "Missing state parameter".to_string())?;

    // Verify state
    let mut states = state.oidc_states.lock().await;
    let (code_verifier, redirect_uri) = match states.remove(state_param) {
        Some(s) => {
            let parts: Vec<&str> = s.state.splitn(2, ':').collect();
            let _orig_state = parts[0].to_string();
            let stored_redirect = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
            (s.code_verifier, stored_redirect)
        }
        None => return Err("Invalid state parameter".to_string()),
    };
    drop(states);

    // Exchange code for token (client_secret_post — exactly like alloy)
    let client = reqwest::Client::new();
    let token_params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &state.config.oidc_redirect_url),
        ("client_id", &state.config.oidc_client_id),
        ("client_secret", &state.config.oidc_client_secret),
        ("code_verifier", &code_verifier),
    ];

    let token_resp = client
        .post(&metadata.token_endpoint)
        .form(&token_params)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {}", e))?;

    let status = token_resp.status();
    let token_body: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|e| format!("Invalid token response (HTTP {}): {}", status.as_u16(), e))?;

    // Check for OAuth2 error in response
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
        return Err(format!("Token endpoint returned error: {} ({})", err, desc));
    }

    // Extract access_token (exactly like alloy)
    let access_token = token_body["access_token"]
        .as_str()
        .ok_or_else(|| "No access_token in token response".to_string())?;

    // Validate token against JWKS (exactly like alloy)
    let _jwt_claims = state
        .jwt_validator
        .validate_token(access_token)
        .await
        .map_err(|e| format!("Token validation failed: {}", e))?;

    // Set cookie and redirect (exactly like alloy)
    let cookie = format!(
        "token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        access_token
    );

    let target = if redirect_uri.is_empty() {
        "/"
    } else {
        &redirect_uri
    };

    let response = axum::response::Response::builder()
        .header("Set-Cookie", cookie)
        .header("Location", target)
        .status(302)
        .body(axum::body::Body::empty())
        .unwrap();

    Ok(response)
}

pub async fn me(
    State(_state): State<Arc<AppState>>,
    claims: axum::extract::Extension<Claims>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "sub": claims.sub,
        "email": claims.email,
        "name": claims.name,
    }))
}
