# Arquitectura MVP (repo)

```
[UI Tauri/React] → invoke(comandos Rust) → docker compose / wpcli / hosts / FS
                                              ↓
                               templates/site-compose → ~/.crea-tu-web/sites/<slug>
```

## Comandos Tauri

| Comando | Rol |
| --- | --- |
| `check_docker` | Detecta Docker + Compose; mensaje ES si falla |
| `list_sites` / `create_site` / `provision_site` | Ciclo de vida + WP-CLI |
| `start_site` / `stop_site` / `delete_site` | Compose; start re-provisiona si falta |
| `get_site_url` / `get_site_admin_url` | `http://dominio.local` (+ `/wp-admin`) |
| `get_hosts_info` / `add_hosts_entry` / `remove_hosts_entry` | Helper hosts (Windows-ready) |
| `get_data_paths` | Diagnóstico de rutas |

## Evento

`provision-progress` — pasos en español durante create/provision (`starting_containers`, `waiting_database`, `core_install`, `platform`, `editor`, `done` / `error`).

## Decisiones bloqueadas

- Sufijo `.local`
- MySQL (no MariaDB)
- Premium solo ZIP del usuario
- Primer target: Windows 11 (MSI/NSIS en `tauri.conf.json`)
