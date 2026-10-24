use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    #[serde(rename = "wordpress")]
    WordPress,
    #[serde(rename = "woocommerce")]
    WooCommerce,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Editor {
    #[serde(rename = "gutenberg")]
    Gutenberg,
    #[serde(rename = "elementor")]
    Elementor,
    #[serde(rename = "generatepress")]
    GeneratePress,
    #[serde(rename = "divi")]
    Divi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SiteStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    /// Dominio local completo, p. ej. `optimizador.local`
    pub domain: String,
    pub platform: Platform,
    pub editor: Editor,
    /// Ruta al directorio del sitio (compose + volúmenes)
    pub path: String,
    pub http_port: u16,
    pub mysql_port: u16,
    pub status: SiteStatus,
    pub admin_user: String,
    pub admin_password: String,
    /// Ruta opcional al ZIP premium (Divi / Elementor Pro / GP Premium)
    pub premium_zip_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSiteRequest {
    pub name: String,
    /// Solo el hostname sin sufijo, p. ej. `optimizador` → `optimizador.local`
    pub domain_label: String,
    pub platform: Platform,
    pub editor: Editor,
    pub premium_zip_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStatus {
    pub available: bool,
    pub version: Option<String>,
    pub compose_available: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsInfo {
    pub path: String,
    pub platform: String,
    pub note: String,
    pub example_entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppStateFile {
    pub sites: Vec<Site>,
    pub next_http_port: u16,
    pub next_mysql_port: u16,
}
