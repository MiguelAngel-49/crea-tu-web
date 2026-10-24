# Arquitectura MVP (repo)

Resumen alineado con el plan del Project store.

```
[UI Tauri/React] → invoke(comandos Rust) → docker compose / hosts / FS
                                              ↓
                               templates/site-compose → ~/.crea-tu-web/sites/<slug>
```

## Comandos Tauri

| Comando | Rol |
| --- | --- |
| `check_docker` | Detecta Docker + Compose; mensaje ES si falla |
| `list_sites` / `create_site` / `start_site` / `stop_site` / `delete_site` | Ciclo de vida |
| `get_site_url` / `get_site_admin_url` | URLs para el plugin opener |
| `get_hosts_info` / `add_hosts_entry` / `remove_hosts_entry` | Helper hosts (Windows-ready) |
| `get_data_paths` | Diagnóstico de rutas |

## Decisiones bloqueadas

- Sufijo `.local`
- MySQL (no MariaDB)
- Premium solo ZIP del usuario
- Primer target de empaquetado: Windows 11 (MSI/NSIS en `tauri.conf.json`)
