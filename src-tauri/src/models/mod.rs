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
    Provisioning,
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
    /// `true` cuando WP-CLI completó core install + plataforma/editor.
    #[serde(default)]
    pub provisioned: bool,
}

impl Site {
    /// URL pública del sitio: `http://dominio.local` o `http://dominio.local:PUERTO`.
    /// Incluye el puerto HTTP mapeado (p. ej. 8080) para que, con hosts escrito,
    /// el navegador llegue a Nginx sin depender del puerto 80 del host (problemático en Windows).
    pub fn public_url(&self) -> String {
        if self.http_port == 80 {
            format!("http://{}", self.domain)
        } else {
            format!("http://{}:{}", self.domain, self.http_port)
        }
    }

    pub fn admin_url(&self) -> String {
        format!("{}/wp-admin", self.public_url())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionProgress {
    pub site_id: String,
    pub step: String,
    pub message: String,
    pub done: bool,
    pub error: Option<String>,
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
