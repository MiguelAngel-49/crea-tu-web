# Plantilla Compose por sitio

Stack MVP de **Crea-tu-Web**:

| Servicio   | Imagen                         | Rol                          |
| ---------- | ------------------------------ | ---------------------------- |
| `nginx`    | `nginx:1.27-alpine`            | Proxy HTTP al PHP-FPM        |
| `wordpress`| `wordpress:6.7-php8.2-fpm`     | WordPress + PHP-FPM          |
| `db`       | `mysql:8.4`                    | MySQL (decisión bloqueada)   |
| `wpcli`    | `wordpress:cli-php8.2`         | WP-CLI (perfil `tools`)      |

## Placeholders

Al crear un sitio, la app sustituye:

- `{{SITE_SLUG}}` — identificador seguro
- `{{DOMAIN}}` — p. ej. `optimizador.local`
- `{{HTTP_PORT}}` / `{{MYSQL_PORT}}` — puertos locales únicos

## Provisión (automática)

La app, tras `docker compose up -d`, ejecuta WP-CLI vía `docker compose run --rm wpcli …`:

1. `wp db check` (espera MySQL)
2. `wp core install --url=http://{{DOMAIN}} …`
3. Plataforma (p. ej. WooCommerce) y editor (Elementor / GeneratePress / Divi desde ZIP)

`PROVISION.md` en cada sitio documenta lo aplicado; ya no es el único camino (antes era manual).

## Premium

Coloca ZIPs legítimos del usuario en `premium/`. La app **nunca** piratea ni redistribuye plugins/temas de pago.
