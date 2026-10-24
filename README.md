# Crea-tu-Web

Aplicación de **escritorio** (Tauri 2) para crear y gestionar sitios **WordPress** 100 % en local, al estilo Local WP.

- UI: React + Vite + TypeScript (español)
- Orquestación: Rust → CLI de Docker + **WP-CLI**
- Runtime por sitio: Docker Compose (Nginx, PHP-FPM, **MySQL**, WP-CLI)
- Dominios locales con sufijo **`.local`** (ej. `optimizador.local`)
- Primer empaquetado prioritario: **Windows 11**

> Plugins/temas **premium** (Divi, Elementor Pro, GP Premium, etc.): solo con ZIP legítimo aportado por el usuario. Esta app **nunca** piratea ni redistribuye binarios de pago.

## Requisitos

| Componente | Notas |
| --- | --- |
| **Docker Desktop** (Windows 11) o Docker Engine + Compose | Obligatorio para provisionar y arrancar sitios |
| Node.js 20+ | Desarrollo frontend |
| Rust (stable) | Backend Tauri |
| WebView2 | Lo incluye/gestiona el instalador en Windows |

Sin Docker, la app arranca igual, crea la carpeta del sitio y muestra un error claro en español; la provisión queda pendiente hasta que Docker esté listo.

## Estructura

```
crea-tu-web/
  src/                 # UI React (lista de sitios + asistente)
  src-tauri/           # Rust: Docker, WP-CLI, hosts, estado
  templates/
    site-compose/      # docker-compose.yml + nginx + carpeta premium/
  docs/                # Notas del repositorio
```

Datos de la app en disco:

- Windows: `%APPDATA%\crea-tu-web\`
- macOS: `~/Library/Application Support/crea-tu-web/`
- Linux: `~/.local/share/crea-tu-web/`

Cada sitio vive en `…/sites/<slug>/` con su `docker-compose.yml`, `.env` y `PROVISION.md` (registro de lo ejecutado).

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

## Flujo al crear un sitio

1. Validar nombre + dominio `*.local`, plataforma y editor.
2. Copiar plantilla Compose, escribir `.env`, copiar ZIP premium si aplica.
3. Intentar entrada en el archivo **hosts** (UAC en Windows 11).
4. `docker compose up -d`.
5. WP-CLI: esperar MySQL → `wp core install` → WooCommerce / Elementor (latest) / GeneratePress / Divi (ZIP).
6. La UI muestra el progreso (`provision-progress`) y el estado «Provisionando…».

**URL local (Windows-friendly):** Nginx publica un puerto alto (p. ej. **8080**). WordPress `siteurl`/`home` y «Abrir sitio» usan `http://nombre.local:8080` (no el :80 implícito), así el navegador llega al contenedor tras escribir hosts sin ocupar el puerto 80 del sistema.

Si falla a mitad, el sitio queda en la lista con «Provisionar» para reintentar.

## MVP: qué hace y qué no

### Funciona

- Listar sitios y asistente de creación (WordPress | WooCommerce × Gutenberg | Elementor | GeneratePress | Divi+ZIP)
- **Provisión automática WP-CLI** al crear (y reintento con «Provisionar» / Arrancar si falta)
- Imagen **WordPress 6.8** (PHP 8.2) — compatible con Elementor latest sin fijar versión
- Arrancar / parar Compose; abrir sitio y WP Admin (`http://dominio.local:PUERTO`)
- Errores en español si Docker falta o WP-CLI falla
- Helper **hosts** (ruta Windows documentada; escritura con aviso UAC)
- Eliminar sitio (compose down + carpeta + hosts)

### Aún stub / siguiente iteración

- HTTPS local con mkcert / CA de confianza (hoy HTTP + `.local:puerto`)
- Elevación UAC embebida para hosts en Windows
- Backup / logs / Adminer en la UI
- Empaquetado firmado

## Hosts (Windows 11)

Ruta: `C:\Windows\System32\drivers\etc\hosts`

```
127.0.0.1    optimizador.local
```

Luego abre `http://optimizador.local:8080` (o el puerto que muestre la app). Sin hosts, el dominio no resuelve (hace falta UAC / admin).

## Licencias

Respeta las licencias de WordPress, WooCommerce, Elementor, GeneratePress, Divi y Docker. Los artefactos premium son responsabilidad del usuario.
