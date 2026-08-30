use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub database_url: PathBuf,
    pub log_level: String,
    pub log_format: String,
    pub stacks_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub oidc_issuer_url: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_redirect_url: String,
}

impl Config {
    pub fn load() -> Self {
        Self {
            host: env_or("HOST", "0.0.0.0"),
            port: env_or_parsed("PORT", 3056),
            data_dir: PathBuf::from(env_or("DATA_DIR", "./data")),
            database_url: PathBuf::from(env_or("DATABASE_URL", "./data/dockpot.db")),
            log_level: env_or("RUST_LOG", "info"),
            log_format: env_or("LOG_FORMAT", "pretty"),
            stacks_dir: PathBuf::from(env_or("STACKS_DIR", "./data/stacks")),
            templates_dir: PathBuf::from(env_or("TEMPLATES_DIR", "./templates")),
            oidc_issuer_url: env_optional("OIDC_ISSUER_URL"),
            oidc_client_id: env_optional("OIDC_CLIENT_ID"),
            oidc_client_secret: env_optional("OIDC_CLIENT_SECRET"),
            oidc_redirect_url: env_or("OIDC_REDIRECT_URL", "http://localhost:3056/auth/callback"),
        }
    }

    pub fn is_dev_mode(&self) -> bool {
        self.oidc_issuer_url.is_none()
    }
}

pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn env_optional(key: &str) -> Option<String> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

pub fn env_or_parsed<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_or_default() {
        assert_eq!(env_or("UNSET_VAR", "def"), "def");
    }

    #[test]
    fn test_env_or_value() {
        std::env::set_var("TST_ENV", "custom");
        assert_eq!(env_or("TST_ENV", "def"), "custom");
        std::env::remove_var("TST_ENV");
    }

    #[test]
    fn test_env_or_parsed_default() {
        assert_eq!(env_or_parsed::<u16>("UNSET_PORT_X", 3056), 3056);
    }

    #[test]
    fn test_env_or_parsed_value() {
        std::env::set_var("TEST_PORT", "8080");
        assert_eq!(env_or_parsed::<u16>("TEST_PORT", 3056), 8080);
        std::env::remove_var("TEST_PORT");
    }

    #[test]
    fn test_env_or_parsed_invalid() {
        std::env::set_var("TEST_BAD", "abc");
        assert_eq!(env_or_parsed::<u16>("TEST_BAD", 3056), 3056);
        std::env::remove_var("TEST_BAD");
    }

    #[test]
    fn test_env_optional_unset() {
        std::env::remove_var("OPT_UNSET");
        assert_eq!(env_optional("OPT_UNSET"), None);
    }

    #[test]
    fn test_env_optional_empty() {
        std::env::set_var("OPT_EMPTY", "");
        assert_eq!(env_optional("OPT_EMPTY"), None);
        std::env::remove_var("OPT_EMPTY");
    }

    #[test]
    fn test_env_optional_value() {
        std::env::set_var("OPT_VAL", "hello");
        assert_eq!(env_optional("OPT_VAL"), Some("hello".to_string()));
        std::env::remove_var("OPT_VAL");
    }

    #[test]
    fn test_is_dev_mode_true_when_no_issuer() {
        let config = Config {
            host: "0.0.0.0".into(),
            port: 3056,
            data_dir: "./data".into(),
            database_url: "./data/dockpot.db".into(),
            log_level: "info".into(),
            log_format: "pretty".into(),
            stacks_dir: "./data/stacks".into(),
            templates_dir: "./templates".into(),
            oidc_issuer_url: None,
            oidc_client_id: None,
            oidc_client_secret: None,
            oidc_redirect_url: "http://localhost:3056/auth/callback".into(),
        };
        assert!(config.is_dev_mode());
    }

    #[test]
    fn test_is_dev_mode_false_when_issuer_set() {
        let config = Config {
            host: "0.0.0.0".into(),
            port: 3056,
            data_dir: "./data".into(),
            database_url: "./data/dockpot.db".into(),
            log_level: "info".into(),
            log_format: "pretty".into(),
            stacks_dir: "./data/stacks".into(),
            templates_dir: "./templates".into(),
            oidc_issuer_url: Some("https://auth.example.com".into()),
            oidc_client_id: Some("client-id".into()),
            oidc_client_secret: Some("secret".into()),
            oidc_redirect_url: "http://localhost:3056/auth/callback".into(),
        };
        assert!(!config.is_dev_mode());
    }
}
