import { useMemo, useState } from "react";
import type { CreateSiteRequest, Editor, Platform } from "../types";

interface Props {
  open: boolean;
  busy: boolean;
  progressMessage?: string | null;
  onClose: () => void;
  onSubmit: (request: CreateSiteRequest) => Promise<void>;
}

const PLATFORMS: { value: Platform; label: string; hint: string }[] = [
  {
    value: "wordpress",
    label: "WordPress",
    hint: "Sitio o blog estándar.",
  },
  {
    value: "woocommerce",
    label: "WooCommerce",
    hint: "Tienda; se instalará WooCommerce vía WP-CLI.",
  },
];

const EDITORS: {
  value: Editor;
  label: string;
  hint: string;
  needsZip: boolean;
}[] = [
  {
    value: "gutenberg",
    label: "Gutenberg",
    hint: "Editor de bloques nativo.",
    needsZip: false,
  },
  {
    value: "elementor",
    label: "Elementor",
    hint: "Free desde wordpress.org; Pro solo con tu ZIP.",
    needsZip: false,
  },
  {
    value: "generatepress",
    label: "GeneratePress",
    hint: "Tema free; GP Premium con tu ZIP.",
    needsZip: false,
  },
  {
    value: "divi",
    label: "Divi",
    hint: "Premium: obligatorio aportar ZIP legítimo.",
    needsZip: true,
  },
];

export function CreateSiteWizard({
  open,
  busy,
  progressMessage,
  onClose,
  onSubmit,
}: Props) {
  const [step, setStep] = useState(0);
  const [name, setName] = useState("");
  const [domainLabel, setDomainLabel] = useState("");
  const [platform, setPlatform] = useState<Platform>("wordpress");
  const [editor, setEditor] = useState<Editor>("gutenberg");
  const [premiumZipPath, setPremiumZipPath] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  const domainPreview = useMemo(() => {
    const label = domainLabel.trim().toLowerCase().replace(/\.local$/i, "");
    return label ? `${label}.local` : "tu-sitio.local";
  }, [domainLabel]);

  const editorMeta = EDITORS.find((e) => e.value === editor)!;
  const needsZip = editorMeta.needsZip;

  if (!open) return null;

  function reset() {
    setStep(0);
    setName("");
    setDomainLabel("");
    setPlatform("wordpress");
    setEditor("gutenberg");
    setPremiumZipPath("");
    setLocalError(null);
  }

  function handleClose() {
    if (busy) return;
    reset();
    onClose();
  }

  function validateStep0(): boolean {
    if (!name.trim()) {
      setLocalError("Indica un nombre para el sitio.");
      return false;
    }
    const label = domainLabel.trim().toLowerCase();
    if (!label || !/^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$/.test(label)) {
      setLocalError(
        "La URL local debe ser un nombre válido (p. ej. optimizador).",
      );
      return false;
    }
    setLocalError(null);
    return true;
  }

  async function handleFinish() {
    if (needsZip && !premiumZipPath.trim()) {
      setLocalError(
        "Divi requiere la ruta a tu ZIP legítimo. No redistribuimos productos de pago.",
      );
      return;
    }
    setLocalError(null);
    try {
      await onSubmit({
        name: name.trim(),
        domainLabel: domainLabel.trim().toLowerCase(),
        platform,
        editor,
        premiumZipPath: premiumZipPath.trim() || null,
      });
      reset();
    } catch {
      /* el padre muestra el error */
    }
  }

  return (
    <div className="modal-backdrop" role="presentation" onClick={handleClose}>
      <div
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="wizard-title"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="modal-head">
          <h2 id="wizard-title">Nuevo sitio</h2>
          <button
            type="button"
            className="icon-close"
            onClick={handleClose}
            aria-label="Cerrar"
            disabled={busy}
          >
            ×
          </button>
        </header>

        <ol className="wizard-steps" aria-label="Pasos">
          <li className={step === 0 ? "active" : step > 0 ? "done" : ""}>
            Nombre y URL
          </li>
          <li className={step === 1 ? "active" : step > 1 ? "done" : ""}>
            Plataforma
          </li>
          <li className={step === 2 ? "active" : ""}>Editor</li>
        </ol>

        {localError ? (
          <p className="form-error" role="alert">
            {localError}
          </p>
        ) : null}

        {step === 0 && (
          <div className="wizard-pane">
            <label className="field">
              <span>Nombre del sitio</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Optimizador SEO"
                autoFocus
              />
            </label>
            <label className="field">
              <span>URL local (.local)</span>
              <div className="domain-input">
                <input
                  value={domainLabel}
                  onChange={(e) =>
                    setDomainLabel(
                      e.target.value.toLowerCase().replace(/\.local$/i, ""),
                    )
                  }
                  placeholder="optimizador"
                  spellCheck={false}
                />
                <span className="suffix">.local</span>
              </div>
            </label>
            <p className="hint">
              Se abrirá como <strong>{domainPreview}</strong> (entrada en hosts
              + puerto local).
            </p>
          </div>
        )}

        {step === 1 && (
          <div className="wizard-pane">
            <p className="hint">¿Qué tipo de producto quieres preparar?</p>
            <div className="choice-grid">
              {PLATFORMS.map((p) => (
                <button
                  key={p.value}
                  type="button"
                  className={`choice ${platform === p.value ? "selected" : ""}`}
                  onClick={() => setPlatform(p.value)}
                >
                  <strong>{p.label}</strong>
                  <span>{p.hint}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="wizard-pane">
            <p className="hint">
              Cómo construirás las páginas. Los premium solo con ZIP tuyo.
            </p>
            <div className="choice-grid">
              {EDITORS.map((e) => (
                <button
                  key={e.value}
                  type="button"
                  className={`choice ${editor === e.value ? "selected" : ""}`}
                  onClick={() => setEditor(e.value)}
                >
                  <strong>{e.label}</strong>
                  <span>{e.hint}</span>
                </button>
              ))}
            </div>
            {(needsZip ||
              editor === "elementor" ||
              editor === "generatepress") && (
              <label className="field" style={{ marginTop: "1rem" }}>
                <span>
                  {needsZip
                    ? "Ruta al ZIP legítimo (obligatorio)"
                    : "ZIP premium opcional (Pro / Premium)"}
                </span>
                <input
                  value={premiumZipPath}
                  onChange={(e) => setPremiumZipPath(e.target.value)}
                  placeholder="C:\Users\Miguel\Downloads\divi.zip"
                  spellCheck={false}
                />
              </label>
            )}
          </div>
        )}

        <footer className="modal-foot">
          {step > 0 ? (
            <button
              type="button"
              className="btn btn-ghost"
              disabled={busy}
              onClick={() => setStep((s) => s - 1)}
            >
              Atrás
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-ghost"
              disabled={busy}
              onClick={handleClose}
            >
              Cancelar
            </button>
          )}
          {step < 2 ? (
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={() => {
                if (step === 0 && !validateStep0()) return;
                setStep((s) => s + 1);
              }}
            >
              Siguiente
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy}
              onClick={handleFinish}
            >
              {busy ? "Creando y provisionando…" : "Crear sitio"}
            </button>
          )}
        </footer>

        {busy && progressMessage ? (
          <p className="wizard-progress" role="status">
            {progressMessage}
          </p>
        ) : null}
      </div>
    </div>
  );
}
