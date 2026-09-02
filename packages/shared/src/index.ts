export type ShareSettings = {
  enabled: boolean;
  path: string;
};

export type AppSettings = {
  version: number;
  downloadShare: ShareSettings;
  uploadShare: ShareSettings;
  preferredAdapterId: string | null;
  port: number;
  maxUploadBytes: number | null;
  maxInboxBytes: number;
  maxInboxFiles: number;
  idleTimeoutMinutes: number | null;
  trustedNetworks: string[];
};

export type NetworkInterfaceInfo = {
  id: string;
  name: string;
  profileName: string;
  address: string;
  prefixLength: number;
  networkId: string;
  category: string;
  profileResolved: boolean;
  preferred: boolean;
};

export type SessionInfo = {
  id: string;
  address: string;
  userAgent: string;
  createdAt: string;
  lastActivity: string;
};

export type TransferInfo = {
  id: string;
  direction: "upload" | "download";
  name: string;
  transferredBytes: number;
  totalBytes: number;
  state: "active" | "complete" | "cancelled" | "failed" | "expired";
  updatedAt: string;
};

export type ServiceStatus = {
  state: "stopped" | "starting" | "running" | "stopping" | "error";
  serviceId: string | null;
  url: string | null;
  accessCode: string | null;
  startedAt: string | null;
  activeTransfers: number;
  sessions: SessionInfo[];
  transfers: TransferInfo[];
  error: string | null;
};

export type FirewallStatus = {
  configured: boolean;
  programPath: string | null;
  port: number | null;
  detail: string;
};

export type AppSnapshot = {
  appVersion: string;
  settings: AppSettings;
  configurationWarning: string | null;
  service: ServiceStatus;
  networks: NetworkInterfaceInfo[];
  firewall: FirewallStatus;
};

export type SessionResponse = {
  serviceId: string;
  csrfToken: string;
  downloadEnabled: boolean;
  uploadEnabled: boolean;
  maxUploadBytes: number | null;
};

export type DownloadEntry = {
  name: string;
  path: string;
  kind: "directory" | "file";
  size: number;
  modifiedAt: string | null;
};

export type DirectoryResponse = {
  path: string;
  query: string;
  entries: DownloadEntry[];
  nextCursor: string | null;
  nextPage: number | null;
};

export type UploadCreated = {
  uploadId: string;
  offset: number;
  totalBytes: number;
  chunkSize: number;
  serviceId: string;
  lastModified: number;
};

export type ApiError = {
  code: string;
  message: string;
};
