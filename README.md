# Crea-tu-Web

Aplicación de **escritorio** (Tauri 2) para crear y gestionar sitios **WordPress** 100 % en local, al estilo Local WP.

- UI: React + Vite + TypeScript (español)
- Orquestación: Rust → CLI de Docker
- Runtime por sitio: Docker Compose (Nginx, PHP-FPM, **MySQL**, WP-CLI)
- Dominios locales con sufijo **`.local`** (ej. `optimizador.local`)
- Primer empaquetado prioritario: **Windows 11**

> Plugins/temas **premium** (Divi, Elementor Pro, GP Premium, etc.): solo con ZIP legítimo aportado por el usuario. Esta app **nunca** piratea ni redistribuye binarios de pago.

## Requisitos

| Componente | Notas |
| --- | --- |
| **Docker Desktop** (Windows 11) o Docker Engine + Compose | Obligatorio para arrancar sitios |
| Node.js 20+ | Desarrollo frontend |
| Rust (stable) | Backend Tauri |
| WebView2 | Lo incluye/gestiona el instalador en Windows |

Sin Docker, la app arranca igual y muestra un error claro en español; no se pueden arrancar contenedores.

## Estructura

```
crea-tu-web/
  src/                 # UI React (lista de sitios + asistente)
  src-tauri/           # Rust: Docker, hosts, estado de sitios
  templates/
    site-compose/      # docker-compose.yml + nginx + carpeta premium/
  docs/                # Notas del repositorio
```

Datos de la app en disco:

- Windows: `%APPDATA%\crea-tu-web\`
- macOS: `~/Library/Application Support/crea-tu-web/`
- Linux: `~/.local/share/crea-tu-web/`

Cada sitio vive en `…/sites/<slug>/` con su `docker-compose.yml` y `.env`.

## Desarrollo

```bash
npm install
npm run tauri dev
```

Solo frontend (sin ventana nativa; los comandos Tauri fallarán):

```bash
npm run dev
```

Build / instalador Windows:

```bash
npm run tauri build
```

## MVP: qué hace y qué no

### Funciona en este esqueleto

- Listar sitios guardados en el estado local
- Asistente de creación: nombre, URL `.local`, plataforma (WordPress | WooCommerce), editor (Gutenberg | Elementor | GeneratePress | Divi + ZIP)
- Generar carpeta del sitio desde `templates/site-compose` (Compose MySQL + Nginx + WP + WP-CLI)
- Arrancar / parar (`docker compose up -d` / `stop`) con mensajes en español si Docker falta
- Abrir sitio y WP Admin en el navegador (URL por puerto local)
- Abstracción de archivo **hosts** (ruta Windows documentada; escritura con error UAC claro)
- Eliminar sitio (compose down + borrar carpeta + quitar hosts si hay permiso)

### Aún stub / siguiente iteración

- Instalación automática WP-CLI (core install, WooCommerce, Elementor/GP) tras el primer arranque — hoy se documenta en `PROVISION.md` del sitio
- HTTPS local con mkcert / CA de confianza
- Abrir por hostname `.local` en lugar del puerto `127.0.0.1`
- Elevación UAC embebida para hosts en Windows
- Backup / logs / Adminer en la UI
- Empaquetado firmado y smoke test en Windows 11 real

## Hosts (Windows 11)

Ruta: `C:\Windows\System32\drivers\etc\hosts`

Entrada de ejemplo:

```
127.0.0.1    optimizador.local
```

La escritura requiere elevación (UAC). Si falla, el sitio se crea igual y la UI muestra el aviso.

## Licencias

Respeta las licencias de WordPress, WooCommerce, Elementor, GeneratePress, Divi y Docker. Los artefactos premium son responsabilidad del usuario.
