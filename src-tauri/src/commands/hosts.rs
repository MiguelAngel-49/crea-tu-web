use crate::models::HostsInfo;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Ruta del archivo hosts según el SO.
/// Windows 11: `C:\Windows\System32\drivers\etc\hosts`
pub fn hosts_file_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/etc/hosts")
    } else {
        PathBuf::from("/etc/hosts")
    }
}

#[tauri::command]
pub fn get_hosts_info() -> HostsInfo {
    let path = hosts_file_path();
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let note = if cfg!(target_os = "windows") {
        "En Windows 11 la escritura en hosts requiere elevación (UAC). \
         Ejecuta Crea-tu-Web como administrador o confirma el diálogo UAC cuando se solicite. \
         Ruta: C:\\Windows\\System32\\drivers\\etc\\hosts"
            .to_string()
    } else {
        "La escritura en /etc/hosts suele requerir privilegios de administrador (sudo)."
            .to_string()
    };

    HostsInfo {
        path: path.display().to_string(),
        platform: platform.to_string(),
        note,
        example_entry: "127.0.0.1\toptimizador.local".to_string(),
    }
}

const MARKER_PREFIX: &str = "# crea-tu-web:";

fn entry_line(domain: &str) -> String {
    format!("127.0.0.1\t{domain}")
}

fn marker_line(domain: &str) -> String {
    format!("{MARKER_PREFIX} {domain}")
}

/// `true` si `hosts` ya mapea el dominio a una IP (línea no comentada).
pub fn has_hosts_entry(domain: &str) -> bool {
    let path = hosts_file_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.lines().any(|l| {
        let t = l.trim();
        !t.starts_with('#') && t.split_whitespace().any(|p| p == domain)
    })
}

/// Añade `127.0.0.1 dominio.local` al archivo hosts si aún no existe.
/// En Windows sin elevación devolverá un error en español claro.
#[tauri::command]
pub fn add_hosts_entry(domain: String) -> Result<(), String> {
    ensure_valid_local_domain(&domain)?;
    let path = hosts_file_path();

    if has_hosts_entry(&domain) {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| map_hosts_write_error(&path, &e))?;

    writeln!(file).map_err(|e| map_hosts_write_error(&path, &e))?;
    writeln!(file, "{}", marker_line(&domain)).map_err(|e| map_hosts_write_error(&path, &e))?;
    writeln!(file, "{}", entry_line(&domain)).map_err(|e| map_hosts_write_error(&path, &e))?;
    Ok(())
}

/// Elimina la entrada marcada por Crea-tu-Web para el dominio dado.
#[tauri::command]
pub fn remove_hosts_entry(domain: String) -> Result<(), String> {
    ensure_valid_local_domain(&domain)?;
    let path = hosts_file_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(map_hosts_write_error(&path, &e)),
    };

    let marker = marker_line(&domain);
    let entry = entry_line(&domain);
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t != marker && t != entry
        })
        .collect();

    fs::write(&path, filtered.join("\n") + "\n").map_err(|e| map_hosts_write_error(&path, &e))?;
    Ok(())
}

fn ensure_valid_local_domain(domain: &str) -> Result<(), String> {
    if !domain.ends_with(".local") {
        return Err("El dominio debe terminar en «.local».".to_string());
    }
    if domain.len() < 8 || domain.contains(' ') || domain.contains('/') {
        return Err("Dominio local no válido.".to_string());
    }
    Ok(())
}

fn map_hosts_write_error(path: &PathBuf, err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => {
            if cfg!(target_os = "windows") {
                format!(
                    "Sin permiso para modificar el archivo hosts ({}). \
                     En Windows 11 ejecuta Crea-tu-Web como administrador (UAC) e inténtalo de nuevo. \
                     Detalle: {err}",
                    path.display()
                )
            } else {
                format!(
                    "Sin permiso para modificar {} (se necesita administrador). Detalle: {err}",
                    path.display()
                )
            }
        }
        _ => format!(
            "No se pudo actualizar el archivo hosts ({}): {err}",
            path.display()
        ),
    }
}
