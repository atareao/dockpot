use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub category: String,
    pub compose: String,
    pub variables: Vec<TemplateVariable>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub default: String,
    pub required: bool,
}

fn t(name: &str, desc: &str, category: &str, compose: &str, vars: Vec<TemplateVariable>) -> Template {
    Template {
        name: name.to_string(),
        description: desc.to_string(),
        category: category.to_string(),
        compose: compose.to_string(),
        variables: vars,
    }
}

fn v(name: &str, desc: &str, default: &str, required: bool) -> TemplateVariable {
    TemplateVariable {
        name: name.to_string(),
        description: desc.to_string(),
        default: default.to_string(),
        required,
    }
}

pub fn get_templates() -> Vec<Template> {
    vec![
        t("nginx", "Nginx web server with static content", "web", r#"services:
  nginx:
    image: nginx:${NGINX_TAG:-alpine}
    container_name: ${STACK_NAME}-nginx
    ports:
      - "${NGINX_HTTP_PORT:-80}:80"
      - "${NGINX_HTTPS_PORT:-443}:443"
    volumes:
      - ${NGINX_CONF_DIR:-./nginx/conf.d}:/etc/nginx/conf.d:ro
      - ${NGINX_HTML_DIR:-./nginx/html}:/usr/share/nginx/html:ro
    restart: unless-stopped
"#, vec![
            v("NGINX_TAG", "Nginx image tag", "alpine", false),
            v("NGINX_HTTP_PORT", "HTTP port", "80", false),
            v("NGINX_HTTPS_PORT", "HTTPS port", "443", false),
            v("NGINX_CONF_DIR", "Config directory", "./nginx/conf.d", false),
            v("NGINX_HTML_DIR", "HTML directory", "./nginx/html", false),
        ]),
        t("postgres", "PostgreSQL database server", "database", r#"services:
  postgres:
    image: postgres:${POSTGRES_TAG:-16-alpine}
    container_name: ${STACK_NAME}-postgres
    environment:
      POSTGRES_DB: ${POSTGRES_DB:-app}
      POSTGRES_USER: ${POSTGRES_USER:-app}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    ports:
      - "${POSTGRES_PORT:-5432}:5432"
    volumes:
      - ${POSTGRES_DATA_DIR:-./postgres/data}:/var/lib/postgresql/data
    restart: unless-stopped
"#, vec![
            v("POSTGRES_TAG", "PostgreSQL image tag", "16-alpine", false),
            v("POSTGRES_DB", "Database name", "app", false),
            v("POSTGRES_USER", "Database user", "app", false),
            v("POSTGRES_PASSWORD", "Database password", "", true),
            v("POSTGRES_PORT", "Port", "5432", false),
            v("POSTGRES_DATA_DIR", "Data directory", "./postgres/data", false),
        ]),
        t("redis", "Redis key-value store", "database", r#"services:
  redis:
    image: redis:${REDIS_TAG:-7-alpine}
    container_name: ${STACK_NAME}-redis
    command: redis-server --appendonly yes --requirepass ${REDIS_PASSWORD:-}
    ports:
      - "${REDIS_PORT:-6379}:6379"
    volumes:
      - ${REDIS_DATA_DIR:-./redis/data}:/data
    restart: unless-stopped
"#, vec![
            v("REDIS_TAG", "Redis image tag", "7-alpine", false),
            v("REDIS_PASSWORD", "Redis password (leave empty for none)", "", false),
            v("REDIS_PORT", "Redis port", "6379", false),
            v("REDIS_DATA_DIR", "Data directory", "./redis/data", false),
        ]),
        t("traefik", "Traefik reverse proxy with HTTPS", "proxy", r#"services:
  traefik:
    image: traefik:${TRAEFIK_TAG:-v3.1}
    container_name: ${STACK_NAME}-traefik
    command:
      - "--api.dashboard=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.letsencrypt.acme.tlschallenge=true"
      - "--certificatesresolvers.letsencrypt.acme.email=${LETSENCRYPT_EMAIL}"
      - "--certificatesresolvers.letsencrypt.acme.storage=/letsencrypt/acme.json"
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ${LETSENCRYPT_DIR:-./traefik/letsencrypt}:/letsencrypt
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.dashboard.rule=Host(`${DASHBOARD_DOMAIN}`)"
      - "traefik.http.routers.dashboard.service=api@internal"
      - "traefik.http.routers.dashboard.middlewares=auth"
    restart: unless-stopped
"#, vec![
            v("TRAEFIK_TAG", "Traefik image tag", "v3.1", false),
            v("LETSENCRYPT_EMAIL", "Email for Let's Encrypt", "", true),
            v("HTTP_PORT", "HTTP port", "80", false),
            v("HTTPS_PORT", "HTTPS port", "443", false),
            v("DASHBOARD_DOMAIN", "Dashboard domain", "traefik.example.com", false),
            v("LETSENCRYPT_DIR", "Let's Encrypt data directory", "./traefik/letsencrypt", false),
        ]),
        t("mariadb", "MariaDB database server", "database", r#"services:
  mariadb:
    image: mariadb:${MARIADB_TAG:-11}
    container_name: ${STACK_NAME}-mariadb
    environment:
      MARIADB_DATABASE: ${MARIADB_DATABASE:-app}
      MARIADB_USER: ${MARIADB_USER:-app}
      MARIADB_PASSWORD: ${MARIADB_PASSWORD}
      MARIADB_ROOT_PASSWORD: ${MARIADB_ROOT_PASSWORD}
    ports:
      - "${MARIADB_PORT:-3306}:3306"
    volumes:
      - ${MARIADB_DATA_DIR:-./mariadb/data}:/var/lib/mysql
    restart: unless-stopped
"#, vec![
            v("MARIADB_TAG", "MariaDB image tag", "11", false),
            v("MARIADB_DATABASE", "Database name", "app", false),
            v("MARIADB_USER", "Database user", "app", false),
            v("MARIADB_PASSWORD", "Database password", "", true),
            v("MARIADB_ROOT_PASSWORD", "Root password", "", true),
            v("MARIADB_PORT", "Port", "3306", false),
            v("MARIADB_DATA_DIR", "Data directory", "./mariadb/data", false),
        ]),
        t("mongodb", "MongoDB document database", "database", r#"services:
  mongodb:
    image: mongo:${MONGO_TAG:-7}
    container_name: ${STACK_NAME}-mongodb
    environment:
      MONGO_INITDB_ROOT_USERNAME: ${MONGO_USER:-admin}
      MONGO_INITDB_ROOT_PASSWORD: ${MONGO_PASSWORD}
    ports:
      - "${MONGO_PORT:-27017}:27017"
    volumes:
      - ${MONGO_DATA_DIR:-./mongodb/data}:/data/db
    restart: unless-stopped
"#, vec![
            v("MONGO_TAG", "MongoDB image tag", "7", false),
            v("MONGO_USER", "Admin username", "admin", false),
            v("MONGO_PASSWORD", "Admin password", "", true),
            v("MONGO_PORT", "Port", "27017", false),
            v("MONGO_DATA_DIR", "Data directory", "./mongodb/data", false),
        ]),
        t("node-app", "Node.js application with PM2", "app", r#"services:
  app:
    build:
      context: ${BUILD_CONTEXT:-.}
      dockerfile: ${DOCKERFILE:-Dockerfile}
    image: ${IMAGE_NAME:-app}:${IMAGE_TAG:-latest}
    container_name: ${STACK_NAME}-app
    environment:
      NODE_ENV: ${NODE_ENV:-production}
      PORT: ${APP_PORT:-3000}
    ports:
      - "${HOST_PORT:-3000}:${APP_PORT:-3000}"
    volumes:
      - ${APP_DATA_DIR:-./app/data}:/app/data
    restart: unless-stopped
"#, vec![
            v("BUILD_CONTEXT", "Docker build context", ".", false),
            v("IMAGE_NAME", "Image name", "app", false),
            v("IMAGE_TAG", "Image tag", "latest", false),
            v("NODE_ENV", "Node environment", "production", false),
            v("HOST_PORT", "Host port", "3000", false),
            v("APP_PORT", "App port", "3000", false),
            v("APP_DATA_DIR", "Data directory", "./app/data", false),
        ]),
        t("portainer", "Portainer Docker management UI", "management", r#"services:
  portainer:
    image: portainer/portainer-ce:${PORTAINER_TAG:-latest}
    container_name: ${STACK_NAME}-portainer
    command: -H unix:///var/run/docker.sock
    ports:
      - "${PORTAINER_PORT:-9000}:9000"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - ${PORTAINER_DATA:-./portainer/data}:/data
    restart: unless-stopped
"#, vec![
            v("PORTAINER_TAG", "Portainer image tag", "latest", false),
            v("PORTAINER_PORT", "Web UI port", "9000", false),
            v("PORTAINER_DATA", "Data directory", "./portainer/data", false),
        ]),
    ]
}

/// Fill a template compose with the given variables
pub fn fill_template(compose: &str, stack_name: &str, vars: &HashMap<String, String>) -> String {
    let mut result = compose.replace("${STACK_NAME}", stack_name);
    for (key, value) in vars {
        result = result.replace(&format!("${{{}}}", key), value);
    }
    result
}