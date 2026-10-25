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
    /// Origen HTTP con puerto mapeado (omite `:80`). Nunca elimina un puerto ≠ 80.
    pub fn http_origin(host: &str, http_port: u16) -> String {
        if http_port == 80 {
            format!("http://{host}")
        } else {
            format!("http://{host}:{http_port}")
        }
    }

    /// URL canónica del producto: `http://dominio.local` o con puerto mapeado.
    /// Incluye el puerto HTTP (p. ej. 8080) para que, con hosts escrito,
    /// el navegador llegue a Nginx sin depender del puerto 80 del host (problemático en Windows).
    pub fn public_url(&self) -> String {
        Self::http_origin(&self.domain, self.http_port)
    }

    /// URL que debe abrir el navegador ahora.
    /// Si `hosts` no tiene el dominio (p. ej. UAC en Windows), usa `127.0.0.1:PUERTO`
    /// (el puerto mapeado se conserva; sin él WP redirige a `:80` → ERR_CONNECTION_REFUSED).
    /// La canónica `.local` sigue en `public_url`.
    pub fn browser_url(&self) -> String {
        if crate::commands::hosts::has_hosts_entry(&self.domain) {
            return self.public_url();
        }
        Self::http_origin("127.0.0.1", self.http_port)
    }

    /// WP Admin sobre la misma base que Abrir sitio (IP+puerto si hosts falla).
    pub fn admin_url(&self) -> String {
        format!("{}/wp-admin/", self.browser_url())
    }
}

#[cfg(test)]
mod tests {
    use super::Site;

    #[test]
    fn http_origin_keeps_non_default_port() {
        assert_eq!(
            Site::http_origin("127.0.0.1", 8083),
            "http://127.0.0.1:8083"
        );
        assert_eq!(
            Site::http_origin("vedemos-oro-de-inversion.local", 8083),
            "http://vedemos-oro-de-inversion.local:8083"
        );
    }

    #[test]
    fn http_origin_omits_only_port_80() {
        assert_eq!(Site::http_origin("127.0.0.1", 80), "http://127.0.0.1");
        assert_eq!(
            Site::http_origin("demo.local", 80),
            "http://demo.local"
        );
    }

    #[test]
    fn admin_path_never_strips_mapped_port_from_origin() {
        let origin = Site::http_origin("127.0.0.1", 8083);
        let admin = format!("{origin}/wp-admin/");
        assert_eq!(admin, "http://127.0.0.1:8083/wp-admin/");
        assert!(
            !admin.starts_with("http://127.0.0.1/wp-admin"),
            "port must not be stripped for IP fallback admin URL"
        );
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
