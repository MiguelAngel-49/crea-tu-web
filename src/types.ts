export type Platform = "wordpress" | "woocommerce";
export type Editor = "gutenberg" | "elementor" | "generatepress" | "divi";

export type SiteStatus =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "error"
  | "unknown";

export interface Site {
  id: string;
  name: string;
  slug: string;
  domain: string;
  platform: Platform;
  editor: Editor;
  path: string;
  httpPort: number;
  mysqlPort: number;
  status: SiteStatus;
  adminUser: string;
  adminPassword: string;
  premiumZipPath?: string | null;
  createdAt: string;
  lastError?: string | null;
}

export interface CreateSiteRequest {
  name: string;
  domainLabel: string;
  platform: Platform;
  editor: Editor;
  premiumZipPath?: string | null;
}

export interface DockerStatus {
  available: boolean;
  version?: string | null;
  composeAvailable: boolean;
  message: string;
}

export interface HostsInfo {
  path: string;
  platform: string;
  note: string;
  exampleEntry: string;
}
