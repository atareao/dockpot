use std::collections::HashMap;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::label_templates;
use crate::state::AppState;

/// `GET /api/templates/labels`
///
/// Loads all label templates and groups them by category.
/// Returns `{ "traefik": [...], "caddy": [...], ... }`.
pub async fn list_label_templates(
    State(app): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let templates = label_templates::get_label_templates(&app.config.templates_dir);

    let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();
    for tpl in templates {
        let entry = serde_json::to_value(&tpl).unwrap_or_default();
        let category = entry
            .get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("other")
            .to_string();
        grouped.entry(category).or_default().push(entry);
    }

    Ok(Json(json!(grouped)))
}

/// Request body for `POST /api/templates/labels/render`
#[derive(Deserialize)]
pub struct RenderLabelRequest {
    pub template: String,
    pub service_name: String,
    pub variables: Option<HashMap<String, String>>,
}

/// `POST /api/templates/labels/render`
///
/// Finds a label template by name, fills in the variables, and returns the
/// resulting labels as `{ "labels": { ... } }`.
pub async fn render_label_template(
    State(app): State<AppState>,
    Json(req): Json<RenderLabelRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let templates = label_templates::get_label_templates(&app.config.templates_dir);

    let tpl = templates
        .into_iter()
        .find(|t| {
            serde_json::to_value(t)
                .ok()
                .and_then(|v| {
                    v.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n == req.template)
                })
                .unwrap_or(false)
        })
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Label template '{}' not found", req.template),
            )
        })?;

    let tpl_value = serde_json::to_value(&tpl).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize template: {e}"),
        )
    })?;

    let labels: HashMap<String, String> = tpl_value
        .get("labels")
        .and_then(|l| l.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Label template has no 'labels' field".to_string(),
            )
        })?;

    let vars = req.variables.unwrap_or_default();
    let filled = label_templates::fill_label_template(&labels, &req.service_name, &vars);

    Ok(Json(json!({ "labels": filled })))
}

/// Mount the label-template routes under `/api/templates/labels*`
pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/api/templates/labels",
            axum::routing::get(list_label_templates),
        )
        .route(
            "/api/templates/labels/render",
            axum::routing::post(render_label_template),
        )
}
