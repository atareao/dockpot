use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub category: String,
    pub compose: String,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default: String,
    pub required: bool,
}

/// Load all templates from YAML files in the given directory.
/// Returns an empty vec if the directory doesn't exist.
pub fn get_templates(dir: &Path) -> Vec<Template> {
    let dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return vec![],
    };

    let mut templates = Vec::new();
    for entry in dir.flatten() {
        let path = entry.path();
        if path
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_yaml::from_str::<Template>(&content) {
                    Ok(tpl) => {
                        tracing::debug!("📄 Loaded template '{}' from {:?}", tpl.name, path);
                        templates.push(tpl);
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  Failed to parse template {:?}: {}", path, e);
                    }
                },
                Err(e) => {
                    tracing::warn!("⚠️  Failed to read template {:?}: {}", path, e);
                }
            }
        }
    }

    templates
}

/// Fill a template compose with the given variables
pub fn fill_template(compose: &str, stack_name: &str, vars: &HashMap<String, String>) -> String {
    let mut result = compose.replace("${STACK_NAME}", stack_name);
    for (key, value) in vars {
        result = result.replace(&format!("${{{}}}", key), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_templates() -> Vec<Template> {
        // Try to find templates dir relative to project root
        let candidates = ["../templates", "../../templates", "templates"];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return get_templates(&p);
            }
        }
        vec![]
    }

    #[test]
    fn test_load_templates_has_nginx() {
        let templates = test_templates();
        assert!(templates.iter().any(|t| t.name == "nginx"));
    }

    #[test]
    fn test_nginx_template_has_variables() {
        let templates = test_templates();
        let nginx = templates.iter().find(|t| t.name == "nginx").unwrap();
        assert!(nginx.compose.contains("nginx:"));
        assert!(!nginx.variables.is_empty());
    }

    #[test]
    fn test_fill_template_replaces_stack_name() {
        let compose = "container_name: ${STACK_NAME}-app\n";
        let vars = HashMap::new();
        let result = fill_template(compose, "myapp", &vars);
        assert_eq!(result, "container_name: myapp-app\n");
    }

    #[test]
    fn test_fill_template_replaces_variables() {
        let compose = "image: nginx:${NGINX_TAG}\nport: ${PORT}";
        let mut vars = HashMap::new();
        vars.insert("NGINX_TAG".into(), "alpine".into());
        vars.insert("PORT".into(), "8080".into());
        let result = fill_template(compose, "test", &vars);
        assert_eq!(result, "image: nginx:alpine\nport: 8080");
    }

    #[test]
    fn test_fill_template_keeps_unset_vars() {
        let compose = "image: ${UNSET_VAR}";
        let vars = HashMap::new();
        let result = fill_template(compose, "test", &vars);
        assert_eq!(result, compose);
    }

    #[test]
    fn test_postgres_has_required_password() {
        let templates = test_templates();
        let pg = templates.iter().find(|t| t.name == "postgres").unwrap();
        let pw_var = pg
            .variables
            .iter()
            .find(|v| v.name == "POSTGRES_PASSWORD")
            .unwrap();
        assert!(pw_var.required);
    }

    #[test]
    fn test_all_templates_have_category() {
        for t in test_templates() {
            assert!(
                !t.category.is_empty(),
                "Template '{}' has no category",
                t.name
            );
        }
    }

    #[test]
    fn test_traefik_has_letsencrypt() {
        let templates = test_templates();
        let traefik = templates.iter().find(|t| t.name == "traefik").unwrap();
        assert!(traefik.compose.contains("letsencrypt"));
    }

    #[test]
    fn test_mariadb_has_two_required() {
        let templates = test_templates();
        let mariadb = templates.iter().find(|t| t.name == "mariadb").unwrap();
        let required: Vec<_> = mariadb.variables.iter().filter(|v| v.required).collect();
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn test_fill_template_multiple_vars() {
        let compose = "image: ${IMAGE}:${TAG}\nport: ${PORT}";
        let mut vars = HashMap::new();
        vars.insert("IMAGE".into(), "nginx".into());
        vars.insert("TAG".into(), "latest".into());
        vars.insert("PORT".into(), "80".into());
        let result = fill_template(compose, "web", &vars);
        assert_eq!(result, "image: nginx:latest\nport: 80");
    }
}
