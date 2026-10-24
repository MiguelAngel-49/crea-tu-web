import type { Site } from "../types";

const PLATFORM_LABEL: Record<Site["platform"], string> = {
  wordpress: "WordPress",
  woocommerce: "WooCommerce",
};

const EDITOR_LABEL: Record<Site["editor"], string> = {
  gutenberg: "Gutenberg",
  elementor: "Elementor",
  generatepress: "GeneratePress",
  divi: "Divi",
};

const STATUS_LABEL: Record<Site["status"], string> = {
  stopped: "Parado",
  starting: "Arrancando…",
  provisioning: "Provisionando…",
  running: "En marcha",
  stopping: "Parando…",
  error: "Error",
  unknown: "Desconocido",
};

interface Props {
  site: Site;
  busy: boolean;
  dockerOk: boolean;
  progressMessage?: string | null;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onOpen: (id: string) => void;
  onAdmin: (id: string) => void;
  onProvision: (id: string) => void;
  onDelete: (id: string) => void;
}

export function SiteCard({
  site,
  busy,
  dockerOk,
  progressMessage,
  onStart,
  onStop,
  onOpen,
  onAdmin,
  onProvision,
  onDelete,
}: Props) {
  const running = site.status === "running";
  const provisioning = site.status === "provisioning" || busy;
  const needsProvision = !site.provisioned;

  return (
    <article className="site-card">
      <header className="site-card-head">
        <div>
          <h2>{site.name}</h2>
          <p className="site-domain">{site.domain}</p>
        </div>
        <span className={`status status-${site.status}`}>
          {STATUS_LABEL[site.status]}
        </span>
      </header>

      <dl className="site-meta">
        <div>
          <dt>Plataforma</dt>
          <dd>{PLATFORM_LABEL[site.platform]}</dd>
        </div>
        <div>
          <dt>Editor</dt>
          <dd>{EDITOR_LABEL[site.editor]}</dd>
        </div>
        <div>
          <dt>Puerto HTTP</dt>
          <dd>{site.httpPort}</dd>
        </div>
        <div>
          <dt>Admin</dt>
          <dd>
            {site.adminUser} /{" "}
            <code className="mono">{site.adminPassword}</code>
          </dd>
        </div>
        <div>
          <dt>WP-CLI</dt>
          <dd>{site.provisioned ? "Provisionado" : "Pendiente"}</dd>
        </div>
      </dl>

      {progressMessage ? (
        <p className="site-progress" role="status">
          {progressMessage}
        </p>
      ) : null}

      {site.lastError ? (
        <p className="site-error" role="alert">
          {site.lastError}
        </p>
      ) : null}

      <div className="site-actions">
        {needsProvision ? (
          <button
            type="button"
            className="btn btn-primary"
            disabled={provisioning || !dockerOk}
            onClick={() => onProvision(site.id)}
          >
            {provisioning ? "Provisionando…" : "Provisionar"}
          </button>
        ) : running ? (
          <button
            type="button"
            className="btn btn-secondary"
            disabled={busy || !dockerOk}
            onClick={() => onStop(site.id)}
          >
            Parar
          </button>
        ) : (
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy || !dockerOk}
            onClick={() => onStart(site.id)}
          >
            Arrancar
          </button>
        )}
        <button
          type="button"
          className="btn btn-ghost"
          disabled={busy || !site.provisioned}
          onClick={() => onOpen(site.id)}
        >
          Abrir sitio
        </button>
        <button
          type="button"
          className="btn btn-ghost"
          disabled={busy || !site.provisioned}
          onClick={() => onAdmin(site.id)}
        >
          WP Admin
        </button>
        <button
          type="button"
          className="btn btn-danger-ghost"
          disabled={busy}
          onClick={() => onDelete(site.id)}
        >
          Eliminar
        </button>
      </div>
    </article>
  );
}
