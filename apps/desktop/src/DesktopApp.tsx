import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { QRCodeSVG } from "qrcode.react";
import type {
  AppSettings,
  AppSnapshot,
  FirewallStatus,
  NetworkInterfaceInfo,
  ServiceStatus,
  ShareSettings,
  TransferInfo,
} from "@dmdc/shared";
import { text } from "./i18n";

type View = "overview" | "shares" | "transfers" | "security" | "diagnostics";
type BusyAction = "start" | "stop" | "save" | "firewall" | "diagnostics" | "command" | null;

const brandIconUrl = new URL("../../../assets/icon.svg", import.meta.url).href;

const emptyService: ServiceStatus = {
  state: "stopped",
  serviceId: null,
  url: null,
  accessCode: null,
  startedAt: null,
  activeTransfers: 0,
  sessions: [],
  transfers: [],
  error: null,
};

const navigation: { id: View; label: string }[] = [
  { id: "overview", label: text.overview },
  { id: "shares", label: text.shares },
  { id: "transfers", label: text.transfers },
  { id: "security", label: text.networkSecurity },
  { id: "diagnostics", label: text.diagnostics },
];

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${new Intl.NumberFormat("de-DE", { maximumFractionDigits: index ? 1 : 0 }).format(value / 1024 ** index)} ${units[index]}`;
}

function formatDateTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("de-DE", { dateStyle: "short", timeStyle: "short" }).format(date);
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return text.unknownError;
}

function serviceHostname(url: string | null): string | null {
  if (!url) return null;
  try {
    return new URL(url).hostname;
  } catch {
    return null;
  }
}

function transferPercentage(transfer: TransferInfo): number | null {
  if (!Number.isFinite(transfer.totalBytes) || transfer.totalBytes <= 0) return null;
  return Math.min(100, Math.max(0, transfer.transferredBytes / transfer.totalBytes * 100));
}

function TransferProgress({ transfer }: { transfer: TransferInfo }) {
  const percentage = transferPercentage(transfer);
  const sizeLabel = percentage === null
    ? text.transferredUnknown(formatBytes(transfer.transferredBytes))
    : `${formatBytes(transfer.transferredBytes)} ${text.of} ${formatBytes(transfer.totalBytes)}`;

  return (
    <div className="transfer-details">
      <div className="transfer-meta">
        <span>{sizeLabel}</span>
        <strong>{percentage === null ? text.unknownTotal : `${Math.round(percentage)} %`}</strong>
      </div>
      <div
        className={`progress-track${percentage === null ? " unknown" : ""}`}
        role="progressbar"
        aria-label={`${transfer.name}: ${sizeLabel}`}
        aria-valuemin={0}
        aria-valuemax={percentage === null ? undefined : 100}
        aria-valuenow={percentage === null ? undefined : Math.round(percentage)}
        aria-valuetext={percentage === null ? sizeLabel : `${Math.round(percentage)} Prozent, ${sizeLabel}`}
      >
        <span style={percentage === null ? undefined : { width: `${percentage}%` }} />
      </div>
    </div>
  );
}

function TransferRow({ transfer, compact = false }: { transfer: TransferInfo; compact?: boolean }) {
  const active = transfer.state === "active";
  return (
    <article className={`transfer-row${compact ? " compact" : ""}`}>
      <div className="transfer-heading">
        <div className="transfer-name">
          <strong title={transfer.name}><bdi className="untrusted-name">{transfer.name}</bdi></strong>
          <span>{transfer.direction === "upload" ? text.fromPhone : text.toPhone}</span>
        </div>
        <span className={`state-label ${transfer.state}`}>{text.transferState(transfer.state)}</span>
      </div>
      {(active || !compact) && <TransferProgress transfer={transfer} />}
      {!active && !compact && <time dateTime={transfer.updatedAt}>{text.updatedAt(formatDateTime(transfer.updatedAt))}</time>}
    </article>
  );
}

function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <div className="empty-state">
      <strong>{title}</strong>
      <p>{description}</p>
    </div>
  );
}

function PageHeading({ eyebrow, title, description }: { eyebrow: string; title: string; description: string }) {
  return (
    <header className="page-heading">
      <p className="eyebrow">{eyebrow}</p>
      <h1>{title}</h1>
      <p>{description}</p>
    </header>
  );
}

function NetworkLabel({ network }: { network?: NetworkInterfaceInfo }) {
  if (!network) return <span>{text.noNetwork}</span>;
  return <span>{network.profileName} · {network.name} · {network.address}/{network.prefixLength} · {network.category}</span>;
}

type ShareEditorProps = {
  title: string;
  description: string;
  value: ShareSettings;
  locked: boolean;
  onChange: (value: ShareSettings) => void;
};

function ShareEditor({ title, description, value, locked, onChange }: ShareEditorProps) {
  async function choose() {
    const selected = await open({ directory: true, multiple: false, title });
    if (typeof selected === "string") onChange({ enabled: true, path: selected });
  }

  return (
    <section className="share-editor">
      <div className="section-title-row">
        <div>
          <p className="eyebrow">{text.share}</p>
          <h2>{title}</h2>
          <p>{description}</p>
        </div>
        <label className="switch">
          <input
            type="checkbox"
            checked={value.enabled}
            disabled={locked}
            aria-label={`${title}: ${value.enabled ? text.active : text.off}`}
            onChange={(event) => onChange({ ...value, enabled: event.target.checked })}
          />
          <span aria-hidden="true" />
          <b>{value.enabled ? text.active : text.off}</b>
        </label>
      </div>
      <div className="path-row">
        <div className="path-value" title={value.path || text.noFolder}>{value.path || text.noFolder}</div>
        <button className="button secondary" type="button" disabled={locked} onClick={choose}>{text.chooseFolder}</button>
      </div>
      {locked && <p className="field-note">{text.shareLocked}</p>}
    </section>
  );
}

export function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [draft, setDraft] = useState<AppSettings | null>(null);
  const [view, setView] = useState<View>("overview");
  const [busyAction, setBusyAction] = useState<BusyAction>(null);
  const [notice, setNotice] = useState<{ kind: "info" | "error"; text: string } | null>(null);
  const snapshotRequest = useRef(0);
  const serviceRequest = useRef(0);
  const snapshotRetryAttempt = useRef(0);
  const snapshotAvailable = useRef(false);

  const refreshSnapshot = useCallback(async (synchronizeDraft = false) => {
    const request = ++snapshotRequest.current;
    try {
      const next = await invoke<AppSnapshot>("get_app_snapshot");
      if (request !== snapshotRequest.current) return;
      snapshotAvailable.current = true;
      setSnapshot(next);
      snapshotRetryAttempt.current = 0;
      setDraft((current) => (synchronizeDraft || current === null ? next.settings : current));
      setNotice((current) => current?.kind === "error" ? null : current);
    } catch (error) {
      if (request !== snapshotRequest.current) return;
      setNotice({ kind: "error", text: errorMessage(error) });
    }
  }, []);

  const refreshService = useCallback(async () => {
    if (!snapshotAvailable.current) {
      await refreshSnapshot(true);
      return;
    }
    const request = ++serviceRequest.current;
    try {
      const service = await invoke<ServiceStatus>("get_service_status");
      if (request !== serviceRequest.current) return;
      setSnapshot((current) => (current ? { ...current, service } : current));
    } catch (error) {
      if (request !== serviceRequest.current) return;
      setNotice({ kind: "error", text: errorMessage(error) });
    }
  }, [refreshSnapshot]);

  useEffect(() => { void refreshSnapshot(); }, [refreshSnapshot]);

  useEffect(() => {
    if (snapshot || notice?.kind !== "error" || snapshotRetryAttempt.current >= 3) return;
    const delay = 1000 * 2 ** snapshotRetryAttempt.current;
    snapshotRetryAttempt.current += 1;
    const timer = window.setTimeout(() => void refreshSnapshot(true), delay);
    return () => window.clearTimeout(timer);
  }, [notice, refreshSnapshot, snapshot]);

  const service = snapshot?.service ?? emptyService;
  const running = service.state === "running";
  const busy = busyAction !== null;

  useEffect(() => {
    if (!running) return;
    let cancelled = false;
    let timer: number | undefined;
    const poll = async () => {
      await refreshService();
      if (!cancelled) timer = window.setTimeout(() => void poll(), 5000);
    };
    timer = window.setTimeout(() => void poll(), 5000);
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [refreshService, running]);

  useEffect(() => {
    let refreshTimer: number | undefined;
    const scheduleServiceRefresh = () => {
      if (refreshTimer !== undefined) return;
      refreshTimer = window.setTimeout(() => {
        refreshTimer = undefined;
        void refreshService();
      }, 200);
    };
    const unlisteners = Promise.all([
      listen("stop-requested", () => void stop(false)),
      listen("quit-requested", () => void quit(false)),
      listen("service-status-changed", scheduleServiceRefresh),
      listen("sessions-changed", scheduleServiceRefresh),
      listen("transfer-updated", scheduleServiceRefresh),
      listen("network-changed", scheduleServiceRefresh),
    ]);
    return () => {
      if (refreshTimer !== undefined) window.clearTimeout(refreshTimer);
      void unlisteners.then((items) => items.forEach((unlisten) => unlisten()));
    };
  }, [refreshService]);

  const selectedNetwork = useMemo(() => {
    if (!snapshot || !draft) return undefined;
    const activeAddress = serviceHostname(service.url);
    return snapshot.networks.find((item) => activeAddress === item.address)
      ?? snapshot.networks.find((item) => item.id === draft.preferredAdapterId)
      ?? snapshot.networks.find((item) => item.preferred)
      ?? snapshot.networks[0];
  }, [snapshot, draft, service.url]);

  const activeTransfers = useMemo(() => service.transfers.filter((transfer) => transfer.state === "active"), [service.transfers]);
  const transferHistory = useMemo(() => service.transfers.filter((transfer) => transfer.state !== "active").slice().reverse(), [service.transfers]);

  async function saveSettings(next = draft, refreshAfterSave = true): Promise<AppSettings | null> {
    if (!next) return null;
    if (running) {
      setNotice({ kind: "info", text: text.settingsLocked });
      return null;
    }
    const saved = await invoke<AppSettings>("save_settings", { settings: next });
    setDraft(saved);
    if (refreshAfterSave) await refreshSnapshot();
    return saved;
  }

  function updateDraft(patch: Partial<AppSettings>) {
    setDraft((current) => (current ? { ...current, ...patch } : current));
  }

  async function saveCurrentSettings() {
    setBusyAction("save");
    setNotice(null);
    try {
      const saved = await saveSettings();
      if (saved) setNotice({ kind: "info", text: text.settingsSaved });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function start() {
    if (!draft) return;
    setBusyAction("start");
    setNotice(null);
    try {
      await saveSettings(draft, false);
      const prepared = await invoke<AppSnapshot>("get_app_snapshot");
      setSnapshot(prepared);
      setDraft(prepared.settings);
      const preparedNetwork = prepared.networks.find((item) => item.id === prepared.settings.preferredAdapterId)
        ?? prepared.networks.find((item) => item.preferred)
        ?? prepared.networks[0];
      if (!prepared.firewall.configured) {
        const continueWithoutRule = await ask(text.firewallWarning, {
          title: text.firewallMissing,
          kind: "warning",
          okLabel: text.startAnyway,
          cancelLabel: text.cancel,
        });
        if (!continueWithoutRule) return;
      }
      let networkApproval: string | null = null;
      let broadShareApproval: string | null = null;
      for (;;) {
        try {
          await invoke("start_service", { networkApproval, broadShareApproval });
          break;
        } catch (error) {
          const message = errorMessage(error);
          if (message.startsWith("NETWORK_UNTRUSTED|")) {
            const [, token, suppliedName] = message.split("|");
            const networkName = suppliedName || preparedNetwork?.name || text.connection;
            const approved = await ask(text.trustNetwork(networkName), {
              title: text.networkConfirm,
              kind: "warning",
              okLabel: text.trustAndStart,
              cancelLabel: text.cancel,
            });
            if (!approved || !token) return;
            networkApproval = token;
            continue;
          }
          if (message.startsWith("BROAD_SHARE|")) {
            const [, token, ...pathParts] = message.split("|");
            const path = pathParts.join("|");
            const approved = await ask(text.broadShareWarning(path), {
              title: text.broadShare,
              kind: "warning",
              okLabel: text.confirmBroadShare,
              cancelLabel: text.cancel,
            });
            if (!approved || !token) return;
            broadShareApproval = token;
            continue;
          }
          throw error;
        }
      }
      await refreshSnapshot(true);
      setView("overview");
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function stop(force = false) {
    setBusyAction("stop");
    setNotice(null);
    try {
      await invoke("stop_service", { force });
      await refreshService();
    } catch (error) {
      const message = errorMessage(error);
      if (message.startsWith("ACTIVE_TRANSFERS")) {
        const accepted = await ask(text.activeStopWarning, {
          title: text.transferRunning,
          kind: "warning",
          okLabel: text.stopAnyway,
          cancelLabel: text.keepRunning,
        });
        if (accepted) await stop(true);
      } else setNotice({ kind: "error", text: message });
    } finally {
      setBusyAction(null);
    }
  }

  async function quit(force = false) {
    try {
      await invoke("quit_app", { force });
    } catch (error) {
      const message = errorMessage(error);
      if (message.startsWith("ACTIVE_TRANSFERS")) {
        const accepted = await ask(text.activeQuitWarning, {
          title: text.quit,
          kind: "warning",
          okLabel: text.quitNow,
          cancelLabel: text.keepRunning,
        });
        if (accepted) await quit(true);
      } else setNotice({ kind: "error", text: message });
    }
  }

  async function configureFirewall() {
    if (!draft || running) return;
    setBusyAction("firewall");
    setNotice(null);
    try {
      const saved = await saveSettings(draft, false);
      if (!saved) return;
      const firewall = await invoke<FirewallStatus>("configure_firewall");
      setSnapshot((current) => (current ? { ...current, firewall } : current));
      setNotice({ kind: "info", text: text.firewallConfigured });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function exportDiagnostics() {
    const destination = await save({
      title: text.diagnosticTitle,
      defaultPath: `DMDC-Diagnose-${new Date().toISOString().slice(0, 10)}.json`,
      filters: [{ name: text.diagnosticFilter, extensions: ["json"] }],
    });
    if (!destination) return;
    setBusyAction("diagnostics");
    setNotice(null);
    try {
      await invoke("export_diagnostics", { destination });
      setNotice({ kind: "info", text: text.diagnosticSaved });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function simpleCommand(command: string, args: Record<string, unknown> = {}) {
    setBusyAction("command");
    setNotice(null);
    try {
      await invoke(command, args);
      await refreshService();
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    } finally {
      setBusyAction(null);
    }
  }

  async function copyAccessCode() {
    if (!service.accessCode) return;
    try {
      await navigator.clipboard.writeText(service.accessCode);
      setNotice({ kind: "info", text: text.codeCopied });
    } catch (error) {
      setNotice({ kind: "error", text: errorMessage(error) });
    }
  }

  if (!snapshot || !draft) {
    return (
      <main className="loading">
        <p>{notice?.kind === "error" ? notice.text : text.preparing}</p>
        {notice?.kind === "error" && <button className="button primary" type="button" onClick={() => { snapshotRetryAttempt.current = 0; void refreshSnapshot(true); }}>{text.retry}</button>}
      </main>
    );
  }

  const currentSnapshot: AppSnapshot = snapshot;
  const currentDraft: AppSettings = draft;
  const sameFolder = currentDraft.downloadShare.enabled
    && currentDraft.uploadShare.enabled
    && currentDraft.downloadShare.path
    && currentDraft.downloadShare.path.localeCompare(currentDraft.uploadShare.path, undefined, { sensitivity: "accent" }) === 0;
  const enabledShares = Number(currentDraft.downloadShare.enabled) + Number(currentDraft.uploadShare.enabled);
  const statusLabel = running ? text.serviceOnline : service.state === "error" ? text.serviceError : text.serviceOffline;

  function renderConnectionPanel() {
    if (!running || !service.url || !service.accessCode) {
      return (
        <section className="connection-panel unavailable">
          <div>
            <p className="eyebrow">{text.connectPhone}</p>
            <h2>{text.connectionAfterStart}</h2>
            <p>{text.connectionAfterStartDescription}</p>
          </div>
          <span className="status-chip offline">{text.serviceOffline}</span>
        </section>
      );
    }
    return (
      <section className="connection-panel">
        <QRCodeSVG value={service.url} size={106} level="M" bgColor="#f4efe5" fgColor="#11100e" />
        <div className="connection-address">
          <p className="eyebrow">{text.connectPhone}</p>
          <h2>{text.openAndCode}</h2>
          <a href={service.url} target="_blank" rel="noreferrer">{service.url}</a>
        </div>
        <div className="access-block">
          <p className="eyebrow">{text.accessCode}</p>
          <strong className="access-code">{service.accessCode.replace(/(\d{4})(\d{4})/, "$1 $2")}</strong>
        </div>
        <div className="connection-actions">
          <button className="button secondary" type="button" onClick={() => void copyAccessCode()}>{text.copyCode}</button>
          <button className="button ghost" type="button" disabled={busy} onClick={() => void simpleCommand("rotate_access_code")}>{text.rotateCode}</button>
        </div>
      </section>
    );
  }

  function renderOverview() {
    return (
      <>
        <PageHeading eyebrow={text.commandCenter} title={text.overview} description={text.overviewDescription} />
        {renderConnectionPanel()}
        <div className="overview-primary-grid">
          <section className="primary-panel">
            <div className="panel-heading"><h2>{text.connectedDevices}</h2><span>{text.deviceCount(service.sessions.length)}</span></div>
            {!service.sessions.length ? (
              <EmptyState title={running ? text.noConnectedDevices : text.serviceNotRunning} description={running ? text.noSessionsDescription : text.devicesAfterStart} />
            ) : (
              <div className="device-list">
                {service.sessions.map((session) => (
                  <article className="device-row" key={session.id}>
                    <div>
                      <strong>{session.userAgent || text.mobileBrowser}</strong>
                      <span>{session.address}</span>
                      <time dateTime={session.lastActivity}>{text.lastActive(formatDateTime(session.lastActivity))}</time>
                    </div>
                    <button className="button secondary small" type="button" disabled={busy} onClick={() => void simpleCommand("revoke_session", { sessionId: session.id })}>{text.disconnect}</button>
                  </article>
                ))}
              </div>
            )}
          </section>
          <section className="primary-panel">
            <div className="panel-heading"><h2>{text.activeTransfers}</h2><span>{text.transferCount(activeTransfers.length)}</span></div>
            {!activeTransfers.length ? (
              <EmptyState title={running ? text.noActiveTransfers : text.serviceNotRunning} description={running ? text.noTransfersDescription : text.transfersAfterStart} />
            ) : (
              <div className="transfer-list">{activeTransfers.map((transfer) => <TransferRow transfer={transfer} key={transfer.id} compact />)}</div>
            )}
          </section>
        </div>
        <div className="overview-summary-grid">
          <section className="summary-panel">
            <div className="summary-heading">
              <div><h2>{text.shares}</h2><span>{text.enabledShareCount(enabledShares)}</span></div>
              <button className="text-button" type="button" onClick={() => setView("shares")}>{text.openSection}</button>
            </div>
            <ul>
              <li><span>{text.downloadTitle}</span><b>{currentDraft.downloadShare.enabled ? text.active : text.off}</b></li>
              <li><span>{text.uploadTitle}</span><b>{currentDraft.uploadShare.enabled ? text.active : text.off}</b></li>
            </ul>
          </section>
          <section className="summary-panel">
            <div className="summary-heading">
              <div><h2>{text.networkSecurity}</h2><span>{currentSnapshot.firewall.configured ? text.firewallReady : text.firewallMissingShort}</span></div>
              <button className="text-button" type="button" onClick={() => setView("security")}>{text.openSection}</button>
            </div>
            <p>{selectedNetwork?.name ?? text.automatic} · {selectedNetwork?.address ?? text.noNetworkShort} · {text.portSummary(currentDraft.port)}</p>
          </section>
        </div>
      </>
    );
  }

  function renderShares() {
    return (
      <>
        <PageHeading eyebrow={text.configuration} title={text.shares} description={text.sharesDescription} />
        {running && <div className="locked-notice"><strong>{text.viewOnlyWhileRunning}</strong><span>{text.sharesRuntimeExplanation}</span></div>}
        {sameFolder && <div className="notice info" role="status">{text.sameFolderWarning}</div>}
        <div className="share-page-grid">
          <ShareEditor title={text.downloadTitle} description={text.downloadDescription} value={currentDraft.downloadShare} locked={running} onChange={(downloadShare) => updateDraft({ downloadShare })} />
          <ShareEditor title={text.uploadTitle} description={text.uploadDescription} value={currentDraft.uploadShare} locked={running} onChange={(uploadShare) => updateDraft({ uploadShare })} />
        </div>
        <footer className="page-actions">
          <span>{running ? text.stopToEdit : text.changesApplyOnStart}</span>
          <button className="button primary" type="button" disabled={busy || running} onClick={() => void saveCurrentSettings()}>{text.saveShares}</button>
        </footer>
      </>
    );
  }

  function renderTransfers() {
    return (
      <>
        <PageHeading eyebrow={text.activity} title={text.transfers} description={text.transfersDescription} />
        <section className="transfer-section">
          <div className="section-title-row simple"><h2>{text.activeTransfers}</h2><span>{text.transferCount(activeTransfers.length)}</span></div>
          {!activeTransfers.length ? (
            <EmptyState title={text.noActiveTransfers} description={running ? text.noTransfersDescription : text.transfersAfterStart} />
          ) : (
            <div className="transfer-list roomy">{activeTransfers.map((transfer) => <TransferRow transfer={transfer} key={transfer.id} />)}</div>
          )}
        </section>
        <section className="transfer-section history">
          <div className="section-title-row simple"><h2>{text.transferHistory}</h2><span>{text.entryCount(transferHistory.length)}</span></div>
          {!transferHistory.length ? (
            <EmptyState title={text.noHistory} description={text.noHistoryDescription} />
          ) : (
            <div className="transfer-list roomy">{transferHistory.map((transfer) => <TransferRow transfer={transfer} key={transfer.id} />)}</div>
          )}
        </section>
      </>
    );
  }

  function renderSecurity() {
    return (
      <>
        <PageHeading eyebrow={text.configuration} title={text.networkSecurity} description={text.networkDescription} />
        {running && <div className="locked-notice"><strong>{text.viewOnlyWhileRunning}</strong><span>{text.networkRuntimeExplanation}</span></div>}
        <section className="settings-section">
          <div className="section-title-row simple"><h2>{text.networkSettings}</h2></div>
          <div className="settings-grid">
            <label>
              <span>{text.networkInterface}</span>
              <select disabled={running} value={currentDraft.preferredAdapterId ?? ""} onChange={(event) => updateDraft({ preferredAdapterId: event.target.value || null })}>
                <option value="">{text.automatic}</option>
                {currentSnapshot.networks.map((network) => <option value={network.id} key={network.id}>{network.profileName} · {network.name} · {network.address}</option>)}
              </select>
              <small>{running ? text.restartFieldLocked : <NetworkLabel network={selectedNetwork} />}</small>
            </label>
            <label>
              <span>{text.tcpPort}</span>
              <input disabled={running} type="number" min={1024} max={65535} value={currentDraft.port} onChange={(event) => updateDraft({ port: Number(event.target.value) })} />
              <small>{running ? text.restartFieldLocked : text.defaultPort}</small>
            </label>
            <label>
              <span>{text.uploadLimit}</span>
              <select disabled={running} value={currentDraft.maxUploadBytes === null ? "unlimited" : String(currentDraft.maxUploadBytes / 1024 ** 3)} onChange={(event) => updateDraft({ maxUploadBytes: event.target.value === "unlimited" ? null : Number(event.target.value) * 1024 ** 3 })}>
                <option value="5">5 GiB</option><option value="20">20 GiB</option><option value="50">50 GiB</option><option value="100">100 GiB</option><option value="unlimited">{text.unlimited}</option>
              </select>
              <small>{running ? text.restartFieldLocked : text.diskReserve}</small>
            </label>
            <label>
              <span>{text.inboxStorageLimit}</span>
              <select disabled={running} value={String(currentDraft.maxInboxBytes / 1024 ** 3)} onChange={(event) => updateDraft({ maxInboxBytes: Number(event.target.value) * 1024 ** 3 })}>
                <option value="25">25 GiB</option><option value="50">50 GiB</option><option value="100">100 GiB</option><option value="250">250 GiB</option>
              </select>
              <small>{running ? text.restartFieldLocked : text.inboxStorageHint}</small>
            </label>
            <label>
              <span>{text.inboxFileLimit}</span>
              <select disabled={running} value={String(currentDraft.maxInboxFiles)} onChange={(event) => updateDraft({ maxInboxFiles: Number(event.target.value) })}>
                <option value="1000">1.000</option><option value="5000">5.000</option><option value="10000">10.000</option><option value="50000">50.000</option>
              </select>
              <small>{running ? text.restartFieldLocked : text.inboxFileHint}</small>
            </label>
            <label>
              <span>{text.automaticStop}</span>
              <select disabled={running} value={currentDraft.idleTimeoutMinutes ?? 0} onChange={(event) => updateDraft({ idleTimeoutMinutes: Number(event.target.value) || null })}>
                <option value={0}>{text.off}</option><option value={30}>{text.after30}</option><option value={60}>{text.after60}</option><option value={240}>{text.after240}</option><option value={720}>{text.after720}</option>
              </select>
              <small>{running ? text.restartFieldLocked : text.activeNeverStops}</small>
            </label>
          </div>
        </section>
        <section className="firewall-section">
          <div>
            <p className="eyebrow">{text.windows}</p><h2>{text.windowsFirewall}</h2><p>{currentSnapshot.firewall.detail}</p>
            {running && <span className="field-note">{text.firewallLocked}</span>}
          </div>
          <button className="button secondary" type="button" disabled={busy || running} onClick={() => void configureFirewall()}>{currentSnapshot.firewall.configured ? text.updateRule : text.setupFirewall}</button>
        </section>
        <footer className="page-actions">
          <span>{running ? text.stopToEdit : text.changesApplyOnStart}</span>
          <button className="button primary" type="button" disabled={busy || running} onClick={() => void saveCurrentSettings()}>{text.saveSettings}</button>
        </footer>
      </>
    );
  }

  function renderDiagnostics() {
    return (
      <>
        <PageHeading eyebrow={text.system} title={text.diagnostics} description={text.diagnosticsDescription} />
        <section className="diagnostics-panel">
          <div><p className="eyebrow">{text.privacy}</p><h2>{text.diagnosticExportTitle}</h2><p>{text.diagnosticPrivacy}</p></div>
          <dl>
            <div><dt>{text.service}</dt><dd>{statusLabel}</dd></div>
            <div><dt>{text.network}</dt><dd>{selectedNetwork ? `${selectedNetwork.name} · ${selectedNetwork.address}` : text.noNetworkShort}</dd></div>
            <div><dt>{text.firewall}</dt><dd>{currentSnapshot.firewall.configured ? text.configured : text.notConfigured}</dd></div>
            <div><dt>{text.version}</dt><dd>{currentDraft.uiVersion}</dd></div>
          </dl>
          <button className="button primary" type="button" disabled={busy} onClick={() => void exportDiagnostics()}>{text.exportDiagnostics}</button>
        </section>
      </>
    );
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <img src={brandIconUrl} alt="" />
          <div><strong>DMDC</strong><span>{text.appSubtitle}</span></div>
        </div>
        <nav aria-label={text.mainNavigation}>
          {navigation.map((item) => (
            <button key={item.id} type="button" className={view === item.id ? "active" : ""} aria-current={view === item.id ? "page" : undefined} onClick={() => setView(item.id)}>
              <span>{item.label}</span>
              {item.id === "transfers" && service.activeTransfers > 0 && <b>{service.activeTransfers}</b>}
            </button>
          ))}
        </nav>
        <p className="sidebar-note">{text.localOnly}</p>
      </aside>
      <div className="workspace">
        <header className="service-bar">
          <div className={`service-state ${running ? "running" : service.state === "error" ? "error" : "stopped"}`}>
            <i aria-hidden="true" /><strong>{statusLabel}</strong><span>{selectedNetwork?.name ?? text.noNetworkShort}</span><span>{selectedNetwork?.address ?? "—"}</span>
          </div>
          <div className="service-counters" aria-label={text.runtimeSummary}><span>{text.deviceCount(service.sessions.length)}</span><span>{text.transferCount(service.activeTransfers)}</span></div>
          {running ? (
            <button className="button danger service-action" type="button" disabled={busy} onClick={() => void stop(false)}>{busyAction === "stop" ? text.stopping : text.stop}</button>
          ) : (
            <button className="button primary service-action" type="button" disabled={busy} onClick={() => void start()}>{busyAction === "start" ? text.starting : text.start}</button>
          )}
        </header>
        <div className="notice-region">
          {notice && <div className={`notice ${notice.kind}`} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</div>}
          {!notice && currentSnapshot.configurationWarning && <div className="notice error" role="alert">{currentSnapshot.configurationWarning}</div>}
          {!notice && !currentSnapshot.configurationWarning && service.error && <div className="notice error" role="alert">{service.error}</div>}
        </div>
        <main className="page-content">
          {view === "overview" && renderOverview()}
          {view === "shares" && renderShares()}
          {view === "transfers" && renderTransfers()}
          {view === "security" && renderSecurity()}
          {view === "diagnostics" && renderDiagnostics()}
        </main>
      </div>
    </div>
  );
}
