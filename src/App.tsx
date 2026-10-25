import { useCallback, useEffect, useState } from "react";
import {
  checkDocker,
  createSite,
  deleteSite,
  listSites,
  onProvisionProgress,
  openSiteInBrowser,
  openWpAdmin,
  provisionSite,
  startSite,
  stopSite,
  toSpanishError,
} from "./api/sites";
import { CreateSiteWizard } from "./components/CreateSiteWizard";
import { DockerBanner } from "./components/DockerBanner";
import { SiteCard } from "./components/SiteCard";
import type {
  CreateSiteRequest,
  DockerStatus,
  ProvisionProgress,
  Site,
} from "./types";
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
  const [progressBySite, setProgressBySite] = useState<
    Record<string, string>
  >({});
  const [wizardProgress, setWizardProgress] = useState<string | null>(null);

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

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onProvisionProgress((progress: ProvisionProgress) => {
      setProgressBySite((prev) => ({
        ...prev,
        [progress.siteId]: progress.message,
      }));
      setWizardProgress(progress.message);
      if (progress.done) {
        if (progress.error) {
          setError(progress.error);
        }
        void refreshSites();
      }
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        /* fuera de Tauri no hay eventos */
      });
    return () => {
      unlisten?.();
    };
  }, [refreshSites]);

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
      setProgressBySite((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  }

  async function handleCreate(request: CreateSiteRequest) {
    setCreating(true);
    setError(null);
    setWizardProgress("Preparando archivos del sitio…");
    try {
      const site = await createSite(request);
      setWizardOpen(false);
      setWizardProgress(null);
      await refreshSites();
      if (site.lastError && !site.provisioned) {
        setError(site.lastError);
      }
    } catch (e) {
      setError(toSpanishError(e));
      await refreshSites();
      throw e;
    } finally {
      setCreating(false);
      setWizardProgress(null);
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
              Crea, provisiona con WP-CLI y abre WordPress en{" "}
              <code>*.local</code>.
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
                <code>tu-marca.local</code>. Al crear, la app arranca Docker e
                instala WordPress con WP-CLI.
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
                  progressMessage={progressBySite[site.id] ?? null}
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
                  onProvision={(id) =>
                    void withBusy(id, async () => {
                      await provisionSite(id);
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
        progressMessage={wizardProgress}
        onClose={() => setWizardOpen(false)}
        onSubmit={handleCreate}
      />
    </div>
  );
}

export default App;
