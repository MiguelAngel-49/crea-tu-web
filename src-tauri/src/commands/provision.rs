use crate::commands::docker::{compose_in_dir, ensure_docker};
use crate::models::{Editor, Platform, ProvisionProgress, Site};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const DB_WAIT: Duration = Duration::from_secs(180);
const WP_FILES_WAIT: Duration = Duration::from_secs(180);
const POLL: Duration = Duration::from_secs(3);

fn emit_progress(app: Option<&AppHandle>, site: &Site, step: &str, message: &str) {
    let Some(app) = app else { return };
    let _ = app.emit(
        "provision-progress",
        ProvisionProgress {
            site_id: site.id.to_string(),
            step: step.to_string(),
            message: message.to_string(),
            done: false,
            error: None,
        },
    );
}

fn emit_done(app: Option<&AppHandle>, site: &Site, error: Option<String>) {
    let Some(app) = app else { return };
    let message = if error.is_some() {
        "Provisión finalizada con error.".to_string()
    } else {
        "Sitio listo.".to_string()
    };
    let _ = app.emit(
        "provision-progress",
        ProvisionProgress {
            site_id: site.id.to_string(),
            step: if error.is_some() { "error" } else { "done" }.to_string(),
            message,
            done: true,
            error,
        },
    );
}

fn map_wp_error(detail: &str) -> String {
    let lower = detail.to_lowercase();
    if lower.contains("error establishing a database connection")
        || lower.contains("error: error establishing")
    {
        return format!(
            "WordPress aún no puede conectar con MySQL. Espera unos segundos e inténtalo de nuevo. Detalle: {detail}"
        );
    }
    if lower.contains("already installed") {
        return "WordPress ya estaba instalado en este volumen.".to_string();
    }
    format!("Error de WP-CLI: {detail}")
}

/// Ejecuta `docker compose run --rm wpcli <args…>` en el directorio del sitio.
pub fn wpcli_run(site_path: &str, args: &[&str]) -> Result<String, String> {
    ensure_docker()?;
    let mut cmd = Command::new("docker");
    cmd.arg("compose")
        .arg("run")
        .arg("--rm")
        .arg("wpcli")
        .args(args)
        .current_dir(site_path);

    let output = cmd.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => {
            "No se encontró Docker. Instala Docker Desktop e inténtalo de nuevo.".to_string()
        }
        _ => format!("Error al ejecutar WP-CLI: {e}"),
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("código {:?}", output.status.code())
        };
        return Err(map_wp_error(&detail));
    }

    Ok(if stdout.is_empty() { stderr } else { stdout })
}

fn wait_for_db(app: Option<&AppHandle>, site: &Site) -> Result<(), String> {
    emit_progress(
        app,
        site,
        "waiting_database",
        "Esperando a que MySQL esté listo…",
    );
    let deadline = Instant::now() + DB_WAIT;
    let mut last_err = String::from("tiempo de espera agotado");

    while Instant::now() < deadline {
        match wpcli_run(&site.path, &["db", "check"]) {
            Ok(_) => return Ok(()),
            Err(e) => {
                last_err = e;
                thread::sleep(POLL);
            }
        }
    }

    Err(format!(
        "MySQL no respondió a tiempo ({DB_WAIT:?}). {last_err}"
    ))
}

fn wait_for_wordpress_files(app: Option<&AppHandle>, site: &Site) -> Result<(), String> {
    emit_progress(
        app,
        site,
        "waiting_wordpress",
        "Esperando a que WordPress copie los archivos del núcleo…",
    );
    let deadline = Instant::now() + WP_FILES_WAIT;
    let mut last_err = String::from("tiempo de espera agotado");

    while Instant::now() < deadline {
        match wpcli_run(&site.path, &["core", "is-installed"]) {
            // Exit 0 = ya instalado (archivos + tablas OK)
            Ok(_) => return Ok(()),
            Err(e) => {
                let lower = e.to_lowercase();
                // Archivos presentes, pendiente de core install
                if lower.contains("not installed") {
                    return Ok(());
                }
                // Volumen aún vacío / entrypoint copiando → seguir esperando
                last_err = e;
                thread::sleep(POLL);
            }
        }
    }

    Err(format!(
        "Los archivos de WordPress no estuvieron listos a tiempo. {last_err}"
    ))
}

fn site_public_url(site: &Site) -> String {
    // Esquema local actual: HTTP + dominio .local (hosts). HTTPS/mkcert llega después.
    format!("http://{}", site.domain)
}

fn ensure_core_installed(app: Option<&AppHandle>, site: &Site) -> Result<(), String> {
    emit_progress(
        app,
        site,
        "core_install",
        "Comprobando instalación de WordPress…",
    );

    match wpcli_run(&site.path, &["core", "is-installed"]) {
        Ok(_) => {
            emit_progress(
                app,
                site,
                "core_install",
                "WordPress ya estaba instalado; se reutiliza.",
            );
            return Ok(());
        }
        Err(e) => {
            let lower = e.to_lowercase();
            if !(lower.contains("not installed") || lower.contains("does not seem to be installed"))
            {
                // If check failed for other reasons, still try install
                emit_progress(
                    app,
                    site,
                    "core_install",
                    "Instalando WordPress (core install)…",
                );
            } else {
                emit_progress(
                    app,
                    site,
                    "core_install",
                    "Instalando WordPress (core install)…",
                );
            }
        }
    }

    let url = site_public_url(site);
    let email = format!("admin@{}", site.domain);
    let title = site.name.as_str();

    wpcli_run(
        &site.path,
        &[
            "core",
            "install",
            &format!("--url={url}"),
            &format!("--title={title}"),
            &format!("--admin_user={}", site.admin_user),
            &format!("--admin_password={}", site.admin_password),
            &format!("--admin_email={email}"),
            "--skip-email",
        ],
    )?;

    // Asegurar siteurl/home por si el contenedor cambió algo
    let _ = wpcli_run(
        &site.path,
        &["option", "update", "siteurl", &url],
    );
    let _ = wpcli_run(&site.path, &["option", "update", "home", &url]);

    Ok(())
}

fn install_platform(app: Option<&AppHandle>, site: &Site) -> Result<(), String> {
    match site.platform {
        Platform::WordPress => {
            emit_progress(
                app,
                site,
                "platform",
                "Plataforma WordPress (sin extras).",
            );
            Ok(())
        }
        Platform::WooCommerce => {
            emit_progress(
                app,
                site,
                "platform",
                "Instalando y activando WooCommerce…",
            );
            wpcli_run(
                &site.path,
                &["plugin", "install", "woocommerce", "--activate"],
            )?;
            Ok(())
        }
    }
}

fn install_from_premium_zip(site: &Site, zip_name: &str) -> Result<(), String> {
    let container_path = format!("/premium/{zip_name}");
    // Intentar como tema; si falla, como plugin (Divi / packs mixtos).
    match wpcli_run(
        &site.path,
        &["theme", "install", &container_path, "--activate"],
    ) {
        Ok(_) => Ok(()),
        Err(theme_err) => wpcli_run(
            &site.path,
            &["plugin", "install", &container_path, "--activate"],
        )
        .map(|_| ())
        .map_err(|plugin_err| {
            format!(
                "No se pudo instalar el ZIP premium «{zip_name}» \
                 (tema: {theme_err}; plugin: {plugin_err}). \
                 Comprueba que el ZIP es legítimo y válido."
            )
        }),
    }
}

fn premium_zip_filename(site: &Site) -> Option<String> {
    let path = site.premium_zip_path.as_ref()?;
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

fn install_editor(app: Option<&AppHandle>, site: &Site) -> Result<(), String> {
    match site.editor {
        Editor::Gutenberg => {
            emit_progress(
                app,
                site,
                "editor",
                "Editor Gutenberg (nativo); nada que instalar.",
            );
            Ok(())
        }
        Editor::Elementor => {
            emit_progress(
                app,
                site,
                "editor",
                "Instalando Elementor (gratis desde wordpress.org)…",
            );
            wpcli_run(
                &site.path,
                &["plugin", "install", "elementor", "--activate"],
            )?;
            if let Some(name) = premium_zip_filename(site) {
                emit_progress(
                    app,
                    site,
                    "editor",
                    "Instalando ZIP premium (Elementor Pro u otro) aportado por ti…",
                );
                install_from_premium_zip(site, &name)?;
            }
            Ok(())
        }
        Editor::GeneratePress => {
            emit_progress(
                app,
                site,
                "editor",
                "Instalando tema GeneratePress (gratis)…",
            );
            wpcli_run(
                &site.path,
                &["theme", "install", "generatepress", "--activate"],
            )?;
            if let Some(name) = premium_zip_filename(site) {
                emit_progress(
                    app,
                    site,
                    "editor",
                    "Instalando ZIP premium (GP Premium u otro) aportado por ti…",
                );
                install_from_premium_zip(site, &name)?;
            }
            Ok(())
        }
        Editor::Divi => {
            let name = premium_zip_filename(site).ok_or_else(|| {
                "Divi requiere un ZIP legítimo. Indica la ruta en el asistente; \
                 Crea-tu-Web nunca descarga ni redistribuye Divi."
                    .to_string()
            })?;
            emit_progress(
                app,
                site,
                "editor",
                &format!("Instalando Divi desde tu ZIP («{name}»)…"),
            );
            install_from_premium_zip(site, &name)?;
            Ok(())
        }
    }
}

/// Arranca el stack Compose y provisiona WordPress con WP-CLI.
pub fn provision_site(app: Option<&AppHandle>, site: &Site) -> Result<(), String> {
    ensure_docker()?;

    emit_progress(
        app,
        site,
        "starting_containers",
        "Arrancando contenedores Docker (puede descargar imágenes la primera vez)…",
    );
    compose_in_dir(&site.path, &["up", "-d"]).map_err(|e| {
        format!("No se pudieron arrancar los contenedores: {e}")
    })?;

    // Dar un respiro al entrypoint de WordPress para copiar archivos
    thread::sleep(Duration::from_secs(5));

    let result: Result<(), String> = (|| {
        wait_for_db(app, site)?;
        wait_for_wordpress_files(app, site)?;
        ensure_core_installed(app, site)?;
        install_platform(app, site)?;
        install_editor(app, site)?;
        Ok(())
    })();

    match &result {
        Ok(()) => emit_done(app, site, None),
        Err(e) => emit_done(app, site, Some(e.clone())),
    }

    result
}
