use std::path::PathBuf;

/// Directorio de datos de la app:
/// - Windows: `%APPDATA%\crea-tu-web`
/// - macOS: `~/Library/Application Support/crea-tu-web`
/// - Linux: `~/.local/share/crea-tu-web`
pub fn app_data_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or_else(|| {
        "No se pudo determinar el directorio de datos de la aplicación.".to_string()
    })?;
    let dir = base.join("crea-tu-web");
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!("No se pudo crear el directorio de datos ({}): {e}", dir.display())
    })?;
    Ok(dir)
}

pub fn sites_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join("sites");
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!("No se pudo crear el directorio de sitios ({}): {e}", dir.display())
    })?;
    Ok(dir)
}

pub fn state_file() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("sites.json"))
}

/// Plantillas Compose empaquetadas junto al binario (en desarrollo: `templates/` en la raíz del repo).
pub fn templates_dir() -> Result<PathBuf, String> {
    // 1) Variable de entorno (útil en tests / empaquetado)
    if let Ok(custom) = std::env::var("CREA_TU_WEB_TEMPLATES") {
        let p = PathBuf::from(custom);
        if p.is_dir() {
            return Ok(p);
        }
    }

    // 2) Relativo al ejecutable (instalación)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join("templates").join("site-compose"),
                parent
                    .join("resources")
                    .join("templates")
                    .join("site-compose"),
            ];
            for c in candidates {
                if c.is_dir() {
                    return Ok(c);
                }
            }
        }
    }

    // 3) Relativo al CARGO_MANIFEST_DIR / cwd (desarrollo)
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let candidates = [
        cwd.join("templates").join("site-compose"),
        cwd.join("..").join("templates").join("site-compose"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("templates")
            .join("site-compose"),
    ];
    for c in candidates {
        if c.is_dir() {
            return Ok(c);
        }
    }

    Err(
        "No se encontró la carpeta de plantillas (templates/site-compose). \
         Define CREA_TU_WEB_TEMPLATES o ejecuta desde la raíz del repositorio."
            .to_string(),
    )
}
