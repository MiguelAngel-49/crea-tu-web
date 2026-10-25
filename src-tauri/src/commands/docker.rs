use crate::models::DockerStatus;
use std::process::Command;

fn run_capture(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!(
                "No se encontró el comando «{program}». Instala Docker Desktop (Windows 11) \
                 o Docker Engine + Compose y reinicia la aplicación."
            ),
            _ => format!("Error al ejecutar {program}: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("código de salida {:?}", output.status.code())
        };
        return Err(map_docker_error(&detail));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn map_docker_error(detail: &str) -> String {
    let lower = detail.to_lowercase();
    if lower.contains("cannot connect")
        || lower.contains("is the docker daemon running")
        || lower.contains("error during connect")
        || lower.contains("dockerdesktop")
        || lower.contains("pipe/dockerdesktop")
        || lower.contains("npipe")
    {
        return format!(
            "Docker no está en ejecución. Abre Docker Desktop en Windows 11 \
             y espera a que el motor esté listo. Detalle: {detail}"
        );
    }
    if lower.contains("permission denied") {
        return format!(
            "Sin permiso para hablar con Docker. En Windows, asegúrate de que tu usuario \
             pertenece al grupo de Docker Desktop. Detalle: {detail}"
        );
    }
    format!("Error de Docker: {detail}")
}

/// Comprueba si el CLI de Docker y Compose responden.
#[tauri::command]
pub fn check_docker() -> DockerStatus {
    match run_capture("docker", &["version", "--format", "{{.Server.Version}}"]) {
        Ok(version) => {
            let compose_ok = run_capture("docker", &["compose", "version"])
                .or_else(|_| run_capture("docker-compose", &["version"]))
                .is_ok();
            if compose_ok {
                DockerStatus {
                    available: true,
                    version: Some(version.clone()),
                    compose_available: true,
                    message: format!("Docker {version} listo (Compose disponible)."),
                }
            } else {
                DockerStatus {
                    available: true,
                    version: Some(version),
                    compose_available: false,
                    message: "Docker responde, pero no se encontró «docker compose». \
                              Actualiza Docker Desktop o instala el plugin Compose."
                        .to_string(),
                }
            }
        }
        Err(message) => DockerStatus {
            available: false,
            version: None,
            compose_available: false,
            message,
        },
    }
}

pub fn ensure_docker() -> Result<(), String> {
    let status = check_docker();
    if status.available && status.compose_available {
        Ok(())
    } else {
        Err(status.message)
    }
}

pub fn compose_in_dir(site_path: &str, args: &[&str]) -> Result<String, String> {
    ensure_docker()?;
    let mut full = vec!["compose"];
    full.extend_from_slice(args);
    let output = Command::new("docker")
        .args(&full)
        .current_dir(site_path)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "No se encontró Docker. Instala Docker Desktop e inténtalo de nuevo.".to_string()
            }
            _ => format!("Error al ejecutar docker compose: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(map_docker_error(&stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Consulta si el compose del sitio tiene contenedores en ejecución.
pub fn site_is_running(site_path: &str) -> Result<bool, String> {
    ensure_docker()?;
    let output = Command::new("docker")
        .args(["compose", "ps", "-q", "--status", "running"])
        .current_dir(site_path)
        .output()
        .map_err(|e| format!("No se pudo consultar el estado del sitio: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(map_docker_error(&stderr));
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}
