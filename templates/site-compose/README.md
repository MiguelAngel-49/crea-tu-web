# Plantilla Compose por sitio

Stack MVP de **Crea-tu-Web**:

| Servicio   | Imagen                         | Rol                          |
| ---------- | ------------------------------ | ---------------------------- |
| `nginx`    | `nginx:1.27-alpine`            | Proxy HTTP al PHP-FPM        |
| `wordpress`| `wordpress:6.8-php8.2-fpm`     | WordPress ≥ 6.8 + PHP-FPM    |
| `db`       | `mysql:8.4`                    | MySQL (decisión bloqueada)   |
| `wpcli`    | `wordpress:cli-php8.2`         | WP-CLI (perfil `tools`)      |

WordPress **6.8+** es necesario para instalar Elementor (latest) sin fijar versión antigua.

## Placeholders

Al crear un sitio, la app sustituye:

- `{{SITE_SLUG}}` — identificador seguro
- `{{DOMAIN}}` — p. ej. `optimizador.local`
- `{{HTTP_PORT}}` / `{{MYSQL_PORT}}` — puertos locales únicos

## URL pública

Nginx publica `{{HTTP_PORT}}:80`. WordPress `siteurl`/`home` usan
`http://{{DOMAIN}}:{{HTTP_PORT}}` (omite `:80` si el puerto es 80), para que con
hosts escrito el navegador llegue al contenedor sin ocupar el puerto 80 del host.

## Provisión (automática)

Tras `docker compose up -d`, la app ejecuta WP-CLI vía `docker compose run --rm wpcli …`:

1. `wp db check` (espera MySQL)
2. `wp core install --url=http://dominio.local:PUERTO …`
3. Plataforma (p. ej. WooCommerce) y editor (Elementor latest / GeneratePress / Divi desde ZIP)

## Premium

Coloca ZIPs legítimos del usuario en `premium/`. La app **nunca** piratea ni redistribuye plugins/temas de pago.
