import { useCallback, useEffect, useState } from "react";
import {
  checkDocker,
  createSite,
  deleteSite,
  listSites,
  openSiteInBrowser,
  openWpAdmin,
  startSite,
  stopSite,
  toSpanishError,
} from "./api/sites";
import { CreateSiteWizard } from "./components/CreateSiteWizard";
import { DockerBanner } from "./components/DockerBanner";
import { SiteCard } from "./components/SiteCard";
import type { CreateSiteRequest, DockerStatus, Site } from "./types";
import "./App.css";

function App() {
  const [sites, setSites] = useState<Site[]>([]);
  const [docker, setDocker] = useState<DockerStatus | null>(null);
  const [dockerLoading, setDockerLoading] = useState(true);
  const [listLoading, setListLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshDocker = useCallback(async () => {
    setDockerLoading(true);
    try {
      setDocker(await checkDocker());
    } catch (e) {
      setDocker({
        available: false,
        composeAvailable: false,
        message: toSpanishError(e),
      });
    } finally {
      setDockerLoading(false);
    }
  }, []);

  const refreshSites = useCallback(async () => {
    setListLoading(true);
    try {
      setSites(await listSites());
      setError(null);
    } catch (e) {
      setError(toSpanishError(e));
    } finally {
      setListLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshDocker();
    void refreshSites();
  }, [refreshDocker, refreshSites]);

  const dockerOk = Boolean(docker?.available && docker?.composeAvailable);

  async function withBusy(id: string, fn: () => Promise<void>) {
    setBusyId(id);
    setError(null);
    try {
      await fn();
      await refreshSites();
    } catch (e) {
      setError(toSpanishError(e));
      await refreshSites();
    } finally {
      setBusyId(null);
    }
  }

  async function handleCreate(request: CreateSiteRequest) {
    setCreating(true);
    setError(null);
    try {
      await createSite(request);
      setWizardOpen(false);
      await refreshSites();
    } catch (e) {
      setError(toSpanishError(e));
      throw e;
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden />
          <div>
            <p className="brand-name">Crea-tu-Web</p>
            <p className="brand-tag">WordPress local</p>
          </div>
        </div>
        <nav className="side-nav">
          <button type="button" className="nav-item active">
            Mis sitios
          </button>
        </nav>
        <p className="sidebar-foot">MVP · Docker · Windows 11</p>
      </aside>

      <div className="main-col">
        <header className="topbar">
          <div>
            <h1>Mis sitios</h1>
            <p className="subtitle">
              Crea, arranca y abre WordPress en tu máquina con dominio{" "}
              <code>.local</code>.
            </p>
          </div>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setWizardOpen(true)}
          >
            Nuevo sitio
          </button>
        </header>

        <DockerBanner
          status={docker}
          loading={dockerLoading}
          onRetry={() => void refreshDocker()}
        />

        {error ? (
          <div className="banner banner-warn" role="alert">
            <p>{error}</p>
            <button
              type="button"
              className="linkish"
              onClick={() => setError(null)}
            >
              Cerrar
            </button>
          </div>
        ) : null}

        <section className="sites-section" aria-live="polite">
          {listLoading ? (
            <p className="empty">Cargando sitios…</p>
          ) : sites.length === 0 ? (
            <div className="empty-state">
              <h2>Aún no hay sitios</h2>
              <p>
                Crea el primero con un nombre y una URL{" "}
                <code>tu-marca.local</code>, elige WordPress o WooCommerce y un
                editor.
              </p>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => setWizardOpen(true)}
              >
                Crear mi primer sitio
              </button>
            </div>
          ) : (
            <div className="sites-grid">
              {sites.map((site) => (
                <SiteCard
                  key={site.id}
                  site={site}
                  busy={busyId === site.id}
                  dockerOk={dockerOk}
                  onStart={(id) =>
                    void withBusy(id, async () => {
                      await startSite(id);
                    })
                  }
                  onStop={(id) =>
                    void withBusy(id, async () => {
                      await stopSite(id);
                    })
                  }
                  onOpen={(id) =>
                    void withBusy(id, async () => {
                      await openSiteInBrowser(id);
                    })
                  }
                  onAdmin={(id) =>
                    void withBusy(id, async () => {
                      await openWpAdmin(id);
                    })
                  }
                  onDelete={(id) => {
                    const siteName =
                      sites.find((s) => s.id === id)?.name ?? "este sitio";
                    if (
                      !window.confirm(
                        `¿Eliminar «${siteName}»? Se pararán los contenedores y se borrará la carpeta local.`,
                      )
                    ) {
                      return;
                    }
                    void withBusy(id, async () => {
                      await deleteSite(id, true);
                    });
                  }}
                />
              ))}
            </div>
          )}
        </section>
      </div>

      <CreateSiteWizard
        open={wizardOpen}
        busy={creating}
        onClose={() => setWizardOpen(false)}
        onSubmit={handleCreate}
      />
    </div>
  );
}

export default App;
