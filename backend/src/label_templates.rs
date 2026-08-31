use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::templates::TemplateVariable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelTemplate {
    pub name: String,
    pub description: String,
    pub category: String,
    pub labels: HashMap<String, String>,
    pub variables: Vec<TemplateVariable>,
}

/// Load all label templates from YAML files under `dir/labels/`.
///
/// Scans recursively through subdirectories (e.g., `templates/labels/traefik/`,
/// `templates/labels/caddy/`). If a YAML file does not specify a `category`,
/// the parent directory name is used as the fallback category.
///
/// Returns an empty vec if the `labels/` subdirectory does not exist.
pub fn get_label_templates(dir: &Path) -> Vec<LabelTemplate> {
    let labels_dir = dir.join("labels");
    let entries = match std::fs::read_dir(&labels_dir) {
        Ok(d) => d,
        Err(_) => {
            tracing::debug!(
                "Labels directory does not exist at {:?}, returning empty",
                labels_dir
            );
            return vec![];
        }
    };

    let mut templates = Vec::new();
    for entry in entries.flatten() {
        collect_yaml_files(&entry.path(), &mut templates);
    }
    templates
}

/// Recursively walk `path` (which may be a directory or a file), collecting
/// label templates from any `.yaml` / `.yml` files found.
fn collect_yaml_files(path: &Path, templates: &mut Vec<LabelTemplate>) {
    if path.is_dir() {
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            collect_yaml_files(&entry.path(), templates);
        }
    } else if is_yaml_file(path) {
        let category_fallback = path
            .parent()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_yaml::from_str::<LabelTemplateYaml>(&content) {
                Ok(raw) => {
                    let category = if raw.category.is_empty() {
                        category_fallback.clone().unwrap_or_default()
                    } else {
                        raw.category
                    };
                    let tpl = LabelTemplate {
                        name: raw.name,
                        description: raw.description,
                        category,
                        labels: raw.labels,
                        variables: raw.variables,
                    };
                    tracing::debug!("📄 Loaded label template '{}' from {:?}", tpl.name, path);
                    templates.push(tpl);
                }
                Err(e) => {
                    tracing::warn!("⚠️  Failed to parse label template {:?}: {}", path, e);
                }
            },
            Err(e) => {
                tracing::warn!("⚠️  Failed to read label template {:?}: {}", path, e);
            }
        }
    }
}

/// Intermediate struct that allows an empty or missing category so we can
/// apply the directory-name fallback.
#[derive(Debug, Deserialize)]
struct LabelTemplateYaml {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    labels: HashMap<String, String>,
    #[serde(default)]
    variables: Vec<TemplateVariable>,
}

fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .map(|e| e == "yaml" || e == "yml")
        .unwrap_or(false)
}

/// Fill a label template by substituting placeholders in both keys **and**
/// values of the labels map.
///
/// - `${{SERVICE_NAME}}` is replaced with `service_name`
/// - `${{VAR_NAME}}` is replaced with `vars["VAR_NAME"]`
///
/// If a variable is not present in `vars`, the placeholder is left as-is.
pub fn fill_label_template(
    labels: &HashMap<String, String>,
    service_name: &str,
    vars: &HashMap<String, String>,
) -> HashMap<String, String> {
    labels
        .iter()
        .map(|(key, value)| {
            let new_key = substitute_placeholders(key, service_name, vars);
            let new_value = substitute_placeholders(value, service_name, vars);
            (new_key, new_value)
        })
        .collect()
}

/// Perform the actual placeholder substitution in a single string.
fn substitute_placeholders(
    input: &str,
    service_name: &str,
    vars: &HashMap<String, String>,
) -> String {
    let after_service = input.replace("${SERVICE_NAME}", service_name);
    let mut result = after_service;
    for (var_name, var_value) in vars {
        result = result.replace(&format!("${{{}}}", var_name), var_value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper: create a temporary directory with a label template YAML and
    /// return the path along with a cleanup token.
    fn create_temp_label_yaml(
        filename: &str,
        subdir: &str,
        yaml_content: &str,
    ) -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let labels_dir = dir.path().join("labels").join(subdir);
        std::fs::create_dir_all(&labels_dir).expect("Failed to create labels subdir");
        let file_path = labels_dir.join(filename);
        std::fs::write(&file_path, yaml_content).expect("Failed to write YAML");
        let base = dir.path().to_path_buf();
        (base, dir)
    }

    #[test]
    fn test_load_label_templates_from_dir() {
        let yaml = r#"
name: traefik-labels
description: "Traefik reverse proxy labels"
category: "traefik"
labels:
  traefik.enable: "true"
  traefik.http.routers.${SERVICE_NAME}.rule: "Host(`${DOMAIN}`)"
variables:
  - name: DOMAIN
    description: "The domain for the service"
    default: "example.com"
    required: true
"#;

        let (path, _tmpdir) = create_temp_label_yaml("traefik.yaml", "traefik", yaml);
        let templates = get_label_templates(&path);

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "traefik-labels");
        assert_eq!(templates[0].description, "Traefik reverse proxy labels");
        assert_eq!(templates[0].category, "traefik");
        assert_eq!(templates[0].labels.len(), 2);
        assert_eq!(templates[0].variables.len(), 1);
    }

    #[test]
    fn test_load_label_templates_fallback_category() {
        // YAML without a category — should use parent directory name
        let yaml = r#"
name: caddy-labels
description: "Caddy reverse proxy labels"
labels:
  caddy.enable: "true"
variables: []
"#;

        let (path, _tmpdir) = create_temp_label_yaml("caddy.yaml", "caddy", yaml);
        let templates = get_label_templates(&path);

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "caddy-labels");
        assert_eq!(templates[0].category, "caddy");
    }

    #[test]
    fn test_load_label_templates_empty_dir_returns_empty() {
        let dir = PathBuf::from("/tmp/nonexistent_labels_dir_xyz");
        let templates = get_label_templates(&dir);
        assert!(templates.is_empty());
    }

    #[test]
    fn test_fill_label_template_replaces_service_name() {
        let mut labels = HashMap::new();
        labels.insert(
            "traefik.http.routers.${SERVICE_NAME}.rule".into(),
            "Host(`${SERVICE_NAME}.example.com`)".into(),
        );

        let vars = HashMap::new();
        let result = fill_label_template(&labels, "myapp", &vars);

        assert_eq!(result.len(), 1);
        let key = result.keys().next().unwrap();
        let val = result.values().next().unwrap();
        assert_eq!(key, "traefik.http.routers.myapp.rule");
        assert_eq!(val, "Host(`myapp.example.com`)");
    }

    #[test]
    fn test_fill_label_template_replaces_vars() {
        let mut labels = HashMap::new();
        labels.insert(
            "traefik.http.routers.${SERVICE_NAME}.rule".into(),
            "Host(`${DOMAIN}`)".into(),
        );

        let mut vars = HashMap::new();
        vars.insert("DOMAIN".into(), "app.example.com".into());

        let result = fill_label_template(&labels, "api", &vars);

        assert_eq!(result.len(), 1);
        let val = result.values().next().unwrap();
        assert_eq!(val, "Host(`app.example.com`)");
    }

    #[test]
    fn test_fill_label_template_replaces_vars_in_key() {
        let mut labels = HashMap::new();
        labels.insert(
            "traefik.http.routers.${SERVICE_NAME}.middlewares".into(),
            "${MIDDLEWARE_NAME}@file".into(),
        );

        let mut vars = HashMap::new();
        vars.insert("MIDDLEWARE_NAME".into(), "auth".into());

        let result = fill_label_template(&labels, "web", &vars);

        let expected_key = "traefik.http.routers.web.middlewares";
        assert!(result.contains_key(expected_key));
        assert_eq!(result[expected_key], "auth@file");
    }

    #[test]
    fn test_fill_label_template_keeps_unset_vars() {
        let mut labels = HashMap::new();
        labels.insert(
            "label.${SERVICE_NAME}.path".into(),
            "/${UNSET_VAR}/path".into(),
        );

        let vars = HashMap::new();
        let result = fill_label_template(&labels, "svc", &vars);

        assert_eq!(result.len(), 1);
        let key = result.keys().next().unwrap();
        let val = result.values().next().unwrap();
        // SERVICE_NAME is replaced, UNSET_VAR stays
        assert_eq!(key, "label.svc.path");
        assert_eq!(val, "/${UNSET_VAR}/path");
    }

    #[test]
    fn test_fill_label_template_multiple_vars() {
        let mut labels = HashMap::new();
        labels.insert(
            "traefik.http.routers.${SERVICE_NAME}.rule".into(),
            "Host(`${DOMAIN}`) && PathPrefix(`/${PREFIX}`)".into(),
        );

        let mut vars = HashMap::new();
        vars.insert("DOMAIN".into(), "example.com".into());
        vars.insert("PREFIX".into(), "api".into());

        let result = fill_label_template(&labels, "gateway", &vars);

        assert_eq!(
            result["traefik.http.routers.gateway.rule"],
            "Host(`example.com`) && PathPrefix(`/api`)"
        );
    }
}
