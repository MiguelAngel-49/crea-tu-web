import type { DockerStatus } from "../types";

interface Props {
  status: DockerStatus | null;
  loading: boolean;
  onRetry: () => void;
}

export function DockerBanner({ status, loading, onRetry }: Props) {
  if (loading && !status) {
    return (
      <div className="banner banner-muted" role="status">
        Comprobando Docker…
      </div>
    );
  }

  if (!status) return null;

  if (status.available && status.composeAvailable) {
    return (
      <div className="banner banner-ok" role="status">
        <span>{status.message}</span>
        <button type="button" className="linkish" onClick={onRetry}>
          Volver a comprobar
        </button>
      </div>
    );
  }

  return (
    <div className="banner banner-warn" role="alert">
      <div>
        <strong>Docker no disponible</strong>
        <p>{status.message}</p>
        <p className="banner-hint">
          En Windows 11 instala y arranca{" "}
          <strong>Docker Desktop</strong>, espera a que el motor esté listo y
          pulsa «Reintentar».
        </p>
      </div>
      <button type="button" className="btn btn-secondary" onClick={onRetry}>
        Reintentar
      </button>
    </div>
  );
}
