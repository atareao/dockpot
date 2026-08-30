use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

use crate::config::Config;
use crate::templates;

pub async fn list_templates(
    State(config): State<Config>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let templates = templates::get_templates(&config.templates_dir);
    Ok(Json(serde_json::json!(templates)))
}

pub async fn get_template(
    State(config): State<Config>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let templates = templates::get_templates(&config.templates_dir);
    let tpl = templates
        .into_iter()
        .find(|t| t.name == name)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Template '{}' not found", name),
            )
        })?;
    Ok(Json(serde_json::json!(tpl)))
}

#[derive(Deserialize)]
pub struct RenderRequest {
    pub template: String,
    pub stack_name: String,
    pub variables: Option<HashMap<String, String>>,
}

pub async fn render_template(
    State(config): State<Config>,
    Json(req): Json<RenderRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let templates = templates::get_templates(&config.templates_dir);
    let tpl = templates
        .into_iter()
        .find(|t| t.name == req.template)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Template '{}' not found", req.template),
            )
        })?;

    let vars = req.variables.unwrap_or_default();
    let compose = templates::fill_template(&tpl.compose, &req.stack_name, &vars);

    Ok(Json(serde_json::json!({
        "name": req.stack_name,
        "compose": compose,
    })))
}
