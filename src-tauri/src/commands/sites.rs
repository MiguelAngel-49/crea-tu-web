use crate::commands::docker::{self, compose_in_dir, site_is_running};
use crate::commands::hosts;
use crate::commands::provision;
use crate::models::{
    AppStateFile, CreateSiteRequest, Editor, Platform, Site, SiteStatus,
};
use crate::paths::{self, sites_dir, state_file, templates_dir};
use chrono::Utc;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
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

fn update_site_in_state(site: &Site) -> Result<(), String> {
    let mut state = load_state()?;
    if let Some(existing) = state.sites.iter_mut().find(|s| s.id == site.id) {
        *existing = site.clone();
    } else {
        state.sites.push(site.clone());
    }
    save_state(&state)
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
    notes.push_str(
        "La app ejecuta esta provisión **automáticamente** al crear el sitio \
         (o al pulsar «Provisionar» / arrancar si quedó pendiente).\n\n",
    );
    let public = site.public_url();
    notes.push_str(&format!(
        "- Sitio: **{}**\n- URL: `{public}`\n- Admin: `{}` / (ver app)\n",
        site.name, site.admin_user
    ));
    notes.push_str(&format!(
        "- Plataforma: {:?}\n- Editor: {:?}\n- Provisionado: {}\n\n",
        site.platform, site.editor, site.provisioned
    ));

    notes.push_str("## Pasos automáticos\n\n");
    notes.push_str("1. `docker compose up -d`\n");
    notes.push_str(
        "2. Esperar MySQL (`wp core is-installed` vía mysqli — evita cliente mysql/TLS)\n",
    );
    notes.push_str(&format!(
        "3. `wp core install --url={public}` (incluye puerto HTTP mapeado si ≠ 80)\n"
    ));
    match site.platform {
        Platform::WooCommerce => {
            notes.push_str("4. `wp plugin install woocommerce --activate`\n");
        }
        Platform::WordPress => {
            notes.push_str("4. (WordPress vanilla — sin plugin de tienda)\n");
        }
    }
    match site.editor {
        Editor::Gutenberg => notes.push_str("5. Gutenberg nativo\n"),
        Editor::Elementor => {
            notes.push_str("5. `wp plugin install elementor --activate` (+ ZIP Pro si lo aportaste)\n")
        }
        Editor::GeneratePress => {
            notes.push_str(
                "5. `wp theme install generatepress --activate` (+ ZIP Premium si lo aportaste)\n",
            )
        }
        Editor::Divi => {
            notes.push_str(
                "5. Instalar Divi desde `/premium/<tu-zip>` (solo ZIP legítimo del usuario)\n",
            )
        }
    }

    notes.push_str("\n## Reintento manual\n\n");
    notes.push_str("```bash\ncd <ruta-del-sitio>\ndocker compose up -d\n");
    notes.push_str("docker compose run --rm wpcli core is-installed || true\n");
    notes.push_str(&format!(
        "docker compose run --rm wpcli core install \\\n  --url={public} --title=\"{}\" \\\n  --admin_user={} --admin_password=*** --admin_email=admin@{} --skip-email\n```\n",
        site.name, site.admin_user, site.domain
    ));

    notes.push_str(
        "\n> Premium: la app **nunca** descarga ni redistribuye Divi / Elementor Pro / GP Premium.\n",
    );
    if let Some(zip) = &site.premium_zip_path {
        notes.push_str(&format!("\nZIP premium indicado: `{zip}`\n"));
    }
    notes
}

#[tauri::command]
pub fn list_sites() -> Result<Vec<Site>, String> {
    let mut state = load_state()?;
    for site in &mut state.sites {
        if site.status == SiteStatus::Provisioning {
            continue;
        }
        if Path::new(&site.path).join("docker-compose.yml").exists() {
            match site_is_running(&site.path) {
                Ok(true) => {
                    site.status = SiteStatus::Running;
                    if site.provisioned {
                        site.last_error = None;
                    }
                }
                Ok(false) => {
                    if site.status != SiteStatus::Error {
                        site.status = SiteStatus::Stopped;
                    }
                }
                Err(e) => {
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
pub fn create_site(app: AppHandle, request: CreateSiteRequest) -> Result<Site, String> {
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

    if request.editor == Editor::Divi
        && request
            .premium_zip_path
            .as_ref()
            .map(|s| s.is_empty())
            .unwrap_or(true)
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
    if state
        .sites
        .iter()
        .any(|s| s.domain == domain || s.slug == slug)
    {
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
    let mut site = Site {
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
        premium_zip_path: request.premium_zip_path.filter(|p| !p.trim().is_empty()),
        created_at: Utc::now(),
        last_error: None,
        provisioned: false,
    };

    apply_placeholders_in_tree(&site_path, &site)?;
    write_env_file(&site)?;

    if let Some(zip) = &site.premium_zip_path {
        let premium_dir = site_path.join("premium");
        fs::create_dir_all(&premium_dir).map_err(|e| e.to_string())?;
        let file_name = Path::new(zip)
            .file_name()
            .ok_or_else(|| "Ruta de ZIP premium no válida.".to_string())?;
        fs::copy(zip, premium_dir.join(file_name))
            .map_err(|e| format!("No se pudo copiar el ZIP premium: {e}"))?;
    }

    let mut hosts_warning: Option<String> = None;
    if let Err(e) = hosts::add_hosts_entry(domain.clone()) {
        hosts_warning = Some(format!(
            "No se pudo actualizar hosts (puede hacer falta UAC en Windows 11): {e}"
        ));
    }

    state.next_http_port = state.next_http_port.saturating_add(1);
    state.next_mysql_port = state.next_mysql_port.saturating_add(1);
    state.sites.push(site.clone());
    save_state(&state)?;

    // Provisión automática si Docker está disponible
    match docker::ensure_docker() {
        Ok(()) => {
            site.status = SiteStatus::Provisioning;
            let _ = update_site_in_state(&site);

            match provision::provision_site(Some(&app), &site) {
                Ok(()) => {
                    site.status = SiteStatus::Running;
                    site.provisioned = true;
                    site.last_error = hosts_warning;
                    fs::write(site_path.join("PROVISION.md"), provision_notes(&site))
                        .map_err(|e| format!("No se pudo escribir PROVISION.md: {e}"))?;
                    update_site_in_state(&site)?;
                    Ok(site)
                }
                Err(e) => {
                    let msg = if let Some(w) = hosts_warning {
                        format!(
                            "Provisión WP-CLI falló: {e} — {w}. Usa «Provisionar» para reintentar."
                        )
                    } else {
                        format!(
                            "Provisión WP-CLI falló: {e}. Usa «Provisionar» para reintentar."
                        )
                    };
                    site.status = SiteStatus::Error;
                    site.provisioned = false;
                    site.last_error = Some(msg);
                    let _ = fs::write(site_path.join("PROVISION.md"), provision_notes(&site));
                    update_site_in_state(&site)?;
                    // Devolvemos el sitio para que aparezca en la lista y se pueda reintentar.
                    Ok(site)
                }
            }
        }
        Err(docker_err) => {
            site.status = SiteStatus::Stopped;
            site.provisioned = false;
            let mut msg = format!(
                "Sitio creado en disco, pero no se pudo provisionar: {docker_err}. \
                 Arranca Docker Desktop y usa «Provisionar»."
            );
            if let Some(w) = hosts_warning {
                msg = format!("{msg} {w}");
            }
            site.last_error = Some(msg.clone());
            let _ = fs::write(site_path.join("PROVISION.md"), provision_notes(&site));
            update_site_in_state(&site)?;
            // Devolvemos Ok con aviso en last_error: el sitio existe; UI muestra el mensaje
            Ok(site)
        }
    }
}

#[tauri::command]
pub fn provision_site(app: AppHandle, id: String) -> Result<Site, String> {
    docker::ensure_docker()?;
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let mut state = load_state()?;
    let site = state
        .sites
        .iter_mut()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;

    site.status = SiteStatus::Provisioning;
    site.last_error = None;
    let snapshot = site.clone();
    save_state(&state)?;

    match provision::provision_site(Some(&app), &snapshot) {
        Ok(()) => {
            let mut state = load_state()?;
            let site = state
                .sites
                .iter_mut()
                .find(|s| s.id == uuid)
                .ok_or_else(|| "Sitio no encontrado.".to_string())?;
            site.status = SiteStatus::Running;
            site.provisioned = true;
            site.last_error = None;
            let result = site.clone();
            let _ = fs::write(
                Path::new(&result.path).join("PROVISION.md"),
                provision_notes(&result),
            );
            save_state(&state)?;
            Ok(result)
        }
        Err(e) => {
            let mut state = load_state()?;
            if let Some(site) = state.sites.iter_mut().find(|s| s.id == uuid) {
                site.status = SiteStatus::Error;
                site.provisioned = false;
                site.last_error = Some(e.clone());
                let _ = fs::write(
                    Path::new(&site.path).join("PROVISION.md"),
                    provision_notes(site),
                );
                let _ = save_state(&state);
            }
            Err(e)
        }
    }
}

#[tauri::command]
pub fn start_site(app: AppHandle, id: String) -> Result<Site, String> {
    docker::ensure_docker()?;
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let mut state = load_state()?;
    let site = state
        .sites
        .iter_mut()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;

    if !site.provisioned {
        site.status = SiteStatus::Provisioning;
        let snapshot = site.clone();
        save_state(&state)?;
        return match provision::provision_site(Some(&app), &snapshot) {
            Ok(()) => {
                let mut state = load_state()?;
                let site = state
                    .sites
                    .iter_mut()
                    .find(|s| s.id == uuid)
                    .ok_or_else(|| "Sitio no encontrado.".to_string())?;
                site.status = SiteStatus::Running;
                site.provisioned = true;
                site.last_error = None;
                let result = site.clone();
                save_state(&state)?;
                Ok(result)
            }
            Err(e) => {
                let mut state = load_state()?;
                if let Some(site) = state.sites.iter_mut().find(|s| s.id == uuid) {
                    site.status = SiteStatus::Error;
                    site.last_error = Some(e.clone());
                    let _ = save_state(&state);
                }
                Err(e)
            }
        };
    }

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
    // URL navegable: `.local` si hosts OK; si no, `127.0.0.1:PUERTO`.
    Ok(site.browser_url())
}

#[tauri::command]
pub fn get_site_admin_url(id: String) -> Result<String, String> {
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let state = load_state()?;
    let site = state
        .sites
        .iter()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;
    Ok(site.admin_url())
}

#[tauri::command]
pub fn get_site_canonical_url(id: String) -> Result<String, String> {
    let uuid = Uuid::parse_str(&id).map_err(|_| "Identificador de sitio no válido.".to_string())?;
    let state = load_state()?;
    let site = state
        .sites
        .iter()
        .find(|s| s.id == uuid)
        .ok_or_else(|| "Sitio no encontrado.".to_string())?;
    Ok(site.public_url())
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

#[allow(dead_code)]
pub fn site_path_for_slug(slug: &str) -> Result<PathBuf, String> {
    Ok(sites_dir()?.join(slug))
}
