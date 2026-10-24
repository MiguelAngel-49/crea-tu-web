use crate::commands::docker::{self, compose_in_dir, site_is_running};
use crate::commands::hosts;
use crate::models::{
    AppStateFile, CreateSiteRequest, Editor, Platform, Site, SiteStatus,
};
use crate::paths::{self, sites_dir, state_file, templates_dir};
use chrono::Utc;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn load_state() -> Result<AppStateFile, String> {
    let path = state_file()?;
    if !path.exists() {
        return Ok(AppStateFile {
            sites: vec![],
            next_http_port: 8080,
            next_mysql_port: 33060,
        });
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer el estado de sitios: {e}"))?;
    serde_json::from_str(&raw)
        .map_err(|e| format!("El archivo de estado está dañado ({path}): {e}", path = path.display()))
}

fn save_state(state: &AppStateFile) -> Result<(), String> {
    let path = state_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(state)
        .map_err(|e| format!("No se pudo serializar el estado: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("No se pudo guardar el estado: {e}"))
}

fn slugify(name: &str) -> String {
    let lower = name.to_lowercase();
    let re = Regex::new(r"[^a-z0-9]+").unwrap();
    let s = re.replace_all(&lower, "-");
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "sitio".to_string()
    } else {
        s.chars().take(48).collect()
    }
}

fn validate_domain_label(label: &str) -> Result<String, String> {
    let label = label.trim().to_lowercase();
    let re = Regex::new(r"^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$").unwrap();
    if !re.is_match(&label) {
        return Err(
            "La etiqueta del dominio solo puede contener letras minúsculas, números y guiones \
             (p. ej. «optimizador» → optimizador.local)."
                .to_string(),
        );
    }
    if label.ends_with(".local") {
        return Err("Introduce solo el nombre sin «.local»; se añade automáticamente.".to_string());
    }
    Ok(label)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn render_placeholders(template: &str, site: &Site) -> String {
    template
        .replace("{{SITE_SLUG}}", &site.slug)
        .replace("{{DOMAIN}}", &site.domain)
        .replace("{{HTTP_PORT}}", &site.http_port.to_string())
        .replace("{{MYSQL_PORT}}", &site.mysql_port.to_string())
        .replace("{{ADMIN_USER}}", &site.admin_user)
        .replace("{{ADMIN_PASSWORD}}", &site.admin_password)
        .replace("{{SITE_NAME}}", &site.name)
}

fn apply_placeholders_in_tree(root: &Path, site: &Site) -> Result<(), String> {
    fn walk(dir: &Path, site: &Site) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, site)?;
                continue;
            }
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let is_text = matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("yml" | "yaml" | "conf" | "md" | "env" | "txt" | "sh")
            ) || name == "Dockerfile"
                || name.starts_with('.');
            if !is_text {
                continue;
            }
            let raw = match fs::read_to_string(&path) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if !raw.contains("{{") {
                continue;
            }
            fs::write(&path, render_placeholders(&raw, site)).map_err(|e| {
                format!("No se pudo actualizar plantilla ({}): {e}", path.display())
            })?;
        }
        Ok(())
    }
    walk(root, site)
}

fn write_env_file(site: &Site) -> Result<(), String> {
    let content = format!(
        "SITE_SLUG={}\nDOMAIN={}\nHTTP_PORT={}\nMYSQL_PORT={}\n\
         WORDPRESS_DB_HOST=db\nWORDPRESS_DB_USER=wordpress\nWORDPRESS_DB_PASSWORD=wordpress\n\
         WORDPRESS_DB_NAME=wordpress\nMYSQL_ROOT_PASSWORD=rootpass\n\
         WP_ADMIN_USER={}\nWP_ADMIN_PASSWORD={}\nWP_ADMIN_EMAIL=admin@{}\nSITE_TITLE={}\n",
        site.slug,
        site.domain,
        site.http_port,
        site.mysql_port,
        site.admin_user,
        site.admin_password,
        site.domain,
        site.name.replace('\n', " ")
    );
    fs::write(Path::new(&site.path).join(".env"), content)
        .map_err(|e| format!("No se pudo escribir .env: {e}"))
}

fn provision_notes(site: &Site) -> String {
    let mut notes = String::from("# Provisión Crea-tu-Web\n\n");
    notes.push_str(&format!("Sitio: {}\nDominio: http://{}\n", site.name, site.domain));
    notes.push_str(&format!("Plataforma: {:?}\nEditor: {:?}\n\n", site.platform, site.editor));
    notes.push_str("Tras el primer `docker compose up -d`, usa WP-CLI en el contenedor `wpcli`:\n");
    notes.push_str("```\ndocker compose run --rm wpcli wp core install \\\n");
    notes.push_str(&format!(
        "  --url=http://{} --title=\"{}\" \\\n  --admin_user={} --admin_password={} --admin_email=admin@{}\n```\n\n",
        site.domain, site.name, site.admin_user, site.admin_password, site.domain
    ));

    match site.platform {
        Platform::WooCommerce => {
            notes.push_str("## WooCommerce\n");
            notes.push_str("```\ndocker compose run --rm wpcli wp plugin install woocommerce --activate\n```\n\n");
        }
        Platform::WordPress => {}
    }

    match site.editor {
        Editor::Gutenberg => {
            notes.push_str("## Editor\nGutenberg (editor de bloques nativo); no hace falta instalar nada.\n");
        }
        Editor::Elementor => {
            notes.push_str("## Elementor (free)\n");
            notes.push_str("```\ndocker compose run --rm wpcli wp plugin install elementor --activate\n```\n");
            notes.push_str("Elementor Pro: aporta tu ZIP legítimo en la app (nunca se redistribuye).\n");
        }
        Editor::GeneratePress => {
            notes.push_str("## GeneratePress (free)\n");
            notes.push_str("```\ndocker compose run --rm wpcli wp theme install generatepress --activate\n```\n");
            notes.push_str("GP Premium: aporta tu ZIP legítimo.\n");
        }
        Editor::Divi => {
            notes.push_str("## Divi (premium)\n");
            notes.push_str(
                "Divi requiere un ZIP legítimo del usuario. Colócalo en `premium/` del sitio \
                 o indícalo en el asistente. La app nunca piratea ni redistribuye Divi.\n",
            );
        }
    }

    if let Some(zip) = &site.premium_zip_path {
        notes.push_str(&format!("\nZIP premium indicado: `{zip}`\n"));
    }

    notes
}

#[tauri::command]
pub fn list_sites() -> Result<Vec<Site>, String> {
    let mut state = load_state()?;
    for site in &mut state.sites {
        if Path::new(&site.path).join("docker-compose.yml").exists() {
            match site_is_running(&site.path) {
                Ok(true) => {
                    site.status = SiteStatus::Running;
                    site.last_error = None;
                }
                Ok(false) => {
                    if site.status != SiteStatus::Error {
                        site.status = SiteStatus::Stopped;
                    }
                }
                Err(e) => {
                    // Docker ausente: no marcamos error permanente; dejamos Unknown
                    site.status = SiteStatus::Unknown;
                    site.last_error = Some(e);
                }
            }
        }
    }
    let _ = save_state(&state);
    Ok(state.sites)
}

#[tauri::command]
pub fn create_site(request: CreateSiteRequest) -> Result<Site, String> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err("El nombre del sitio es obligatorio.".to_string());
    }
    if name.len() > 80 {
        return Err("El nombre del sitio es demasiado largo (máx. 80 caracteres).".to_string());
    }

    let label = validate_domain_label(&request.domain_label)?;
    let domain = format!("{label}.local");
    let slug = slugify(&label);

    // Premium ZIP obligatorio para Divi; opcional para otros
    if request.editor == Editor::Divi && request.premium_zip_path.as_ref().map(|s| s.is_empty()).unwrap_or(true)
    {
        return Err(
            "Divi es un producto premium: debes indicar la ruta a tu ZIP legítimo. \
             Crea-tu-Web nunca descarga ni redistribuye Divi."
                .to_string(),
        );
    }
    if let Some(zip) = &request.premium_zip_path {
        if !zip.is_empty() && !Path::new(zip).is_file() {
            return Err(format!(
                "No se encontró el ZIP premium en «{zip}». Comprueba la ruta."
            ));
        }
    }

    let mut state = load_state()?;
    if state.sites.iter().any(|s| s.domain == domain || s.slug == slug) {
        return Err(format!(
            "Ya existe un sitio con el dominio «{domain}» o el identificador «{slug}»."
        ));
    }

    let site_path = sites_dir()?.join(&slug);
    if site_path.exists() {
        return Err(format!(
            "Ya existe la carpeta del sitio en {}.",
            site_path.display()
        ));
    }

    let templates = templates_dir()?;
    copy_dir_recursive(&templates, &site_path)?;

    let admin_password = format!("ctw-{}", &Uuid::new_v4().to_string()[..8]);
    let site = Site {
        id: Uuid::new_v4(),
        name: name.to_string(),
        slug: slug.clone(),
        domain: domain.clone(),
        platform: request.platform,
        editor: request.editor,
        path: site_path.display().to_string(),
        http_port: state.next_http_port,
        mysql_port: state.next_mysql_port,
        status: SiteStatus::Stopped,
        admin_user: "admin".to_string(),
        admin_password,
        premium_zip_path: request
            .premium_zip_path
            .filter(|p| !p.trim().is_empty()),
        created_at: Utc::now(),
        last_error: None,
    };

    // Sustituir placeholders en compose, nginx, etc.
    apply_placeholders_in_tree(&site_path, &site)?;
    write_env_file(&site)?;
    fs::write(site_path.join("PROVISION.md"), provision_notes(&site))
        .map_err(|e| format!("No se pudo escribir PROVISION.md: {e}"))?;

    // Copiar ZIP premium si se indicó
    if let Some(zip) = &site.premium_zip_path {
        let premium_dir = site_path.join("premium");
        fs::create_dir_all(&premium_dir).map_err(|e| e.to_string())?;
        let file_name = Path::new(zip)
            .file_name()
            .ok_or_else(|| "Ruta de ZIP premium no válida.".to_string())?;
        fs::copy(zip, premium_dir.join(file_name))
            .map_err(|e| format!("No se pudo copiar el ZIP premium: {e}"))?;
    }

    // Intentar entrada hosts (puede fallar sin elevación; no abortamos la creación)
    let mut site = site;
    if let Err(e) = hosts::add_hosts_entry(domain.clone()) {
        site.last_error = Some(format!(
            "Sitio creado, pero no se pudo actualizar hosts: {e}"
        ));
    }

    state.next_http_port = state.next_http_port.saturating_add(1);
    state.next_mysql_port = state.next_mysql_port.saturating_add(1);
    state.sites.push(site.clone());
    save_state(&state)?;
    Ok(site)
}

#[tauri::command]
pub fn start_site(id: String) -> Result<Site, String> {
    docker::ensure_docker()?;
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let mut state = load_state()?;
    let site = state
        .sites
        .iter_mut()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;

    site.status = SiteStatus::Starting;
    let path = site.path.clone();
    match compose_in_dir(&path, &["up", "-d"]) {
        Ok(_) => {
            site.status = SiteStatus::Running;
            site.last_error = None;
        }
        Err(e) => {
            site.status = SiteStatus::Error;
            site.last_error = Some(e.clone());
            let _ = save_state(&state);
            return Err(e);
        }
    }
    let result = site.clone();
    save_state(&state)?;
    Ok(result)
}

#[tauri::command]
pub fn stop_site(id: String) -> Result<Site, String> {
    docker::ensure_docker()?;
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let mut state = load_state()?;
    let site = state
        .sites
        .iter_mut()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;

    site.status = SiteStatus::Stopping;
    let path = site.path.clone();
    match compose_in_dir(&path, &["stop"]) {
        Ok(_) => {
            site.status = SiteStatus::Stopped;
            site.last_error = None;
        }
        Err(e) => {
            site.status = SiteStatus::Error;
            site.last_error = Some(e.clone());
            let _ = save_state(&state);
            return Err(e);
        }
    }
    let result = site.clone();
    save_state(&state)?;
    Ok(result)
}

#[tauri::command]
pub fn delete_site(id: String, remove_volumes: bool) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let mut state = load_state()?;
    let idx = state
        .sites
        .iter()
        .position(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;
    let site = state.sites.remove(idx);

    // Intentar bajar contenedores si Docker está disponible
    if Path::new(&site.path).join("docker-compose.yml").exists() {
        let mut args = vec!["down"];
        if remove_volumes {
            args.push("-v");
        }
        let _ = compose_in_dir(&site.path, &args);
    }

    let _ = hosts::remove_hosts_entry(site.domain.clone());

    if Path::new(&site.path).exists() {
        fs::remove_dir_all(&site.path).map_err(|e| {
            format!(
                "No se pudo borrar la carpeta del sitio ({}): {e}",
                site.path
            )
        })?;
    }

    save_state(&state)?;
    Ok(())
}

#[tauri::command]
pub fn get_site_url(id: String) -> Result<String, String> {
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let state = load_state()?;
    let site = state
        .sites
        .iter()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;
    // MVP: HTTP por puerto mapeado; HTTPS/mkcert llega más adelante
    Ok(format!("http://127.0.0.1:{}", site.http_port))
}

#[tauri::command]
pub fn get_site_admin_url(id: String) -> Result<String, String> {
    let url = get_site_url(id)?;
    Ok(format!("{url}/wp-admin"))
}

#[tauri::command]
pub fn get_data_paths() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "appData": paths::app_data_dir()?.display().to_string(),
        "sites": sites_dir()?.display().to_string(),
        "stateFile": state_file()?.display().to_string(),
        "templates": templates_dir().map(|p| p.display().to_string()).unwrap_or_else(|e| e),
    }))
}

/// Utilidad interna: genera una contraseña legible (expuesta solo si hace falta regenerar).
#[allow(dead_code)]
pub fn site_path_for_slug(slug: &str) -> Result<PathBuf, String> {
    Ok(sites_dir()?.join(slug))
}
