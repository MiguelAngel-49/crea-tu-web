import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  CreateSiteRequest,
  DockerStatus,
  HostsInfo,
  ProvisionProgress,
  Site,
} from "../types";

export async function checkDocker(): Promise<DockerStatus> {
  return invoke("check_docker");
}

export async function listSites(): Promise<Site[]> {
  return invoke("list_sites");
}

export async function createSite(request: CreateSiteRequest): Promise<Site> {
  return invoke("create_site", { request });
}

export async function provisionSite(id: string): Promise<Site> {
  return invoke("provision_site", { id });
}

export async function startSite(id: string): Promise<Site> {
  return invoke("start_site", { id });
}

export async function stopSite(id: string): Promise<Site> {
  return invoke("stop_site", { id });
}

export async function deleteSite(
  id: string,
  removeVolumes = true,
): Promise<void> {
  return invoke("delete_site", { id, removeVolumes });
}

export async function getSiteUrl(id: string): Promise<string> {
  return invoke("get_site_url", { id });
}

export async function getSiteAdminUrl(id: string): Promise<string> {
  return invoke("get_site_admin_url", { id });
}

export async function getHostsInfo(): Promise<HostsInfo> {
  return invoke("get_hosts_info");
}

export async function openSiteInBrowser(id: string): Promise<void> {
  const url = await getSiteUrl(id);
  await openUrl(url);
}

export async function openWpAdmin(id: string): Promise<void> {
  const url = await getSiteAdminUrl(id);
  await openUrl(url);
}

export async function onProvisionProgress(
  handler: (progress: ProvisionProgress) => void,
): Promise<UnlistenFn> {
  return listen<ProvisionProgress>("provision-progress", (event) => {
    handler(event.payload);
  });
}

/** Mensaje amigable si invoke falla fuera de Tauri (p. ej. solo Vite). */
export function toSpanishError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  try {
    return JSON.stringify(err);
  } catch {
    return "Ha ocurrido un error inesperado.";
  }
}
