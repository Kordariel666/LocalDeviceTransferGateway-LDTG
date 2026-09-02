import { QRCodeSVG } from "qrcode.react";
import type {
  AppSettings,
  AppSnapshot,
  NetworkInterfaceInfo,
  ServiceStatus,
  TransferInfo,
} from "@dmdc/shared";
import {
  EmptyState,
  NetworkLabel,
  PageHeading,
  ShareEditor,
  TransferRow,
} from "../components/DesktopComponents";
import type { DraftValidationErrors } from "../settingsDraft";
import { text } from "../i18n";
import { formatDateTime } from "../presentation";

type Command = (command: string, args?: Record<string, unknown>) => Promise<void>;

type OverviewPageProps = {
  service: ServiceStatus;
  running: boolean;
  busy: boolean;
  activeTransfers: TransferInfo[];
  enabledShares: number;
  draft: AppSettings;
  snapshot: AppSnapshot;
  selectedNetwork?: NetworkInterfaceInfo;
  onCommand: Command;
  onCopyAccessCode: () => Promise<void>;
  onNavigate: (view: "shares" | "security") => void;
};

function ConnectionPanel({
  service,
  running,
  busy,
  onCommand,
  onCopyAccessCode,
}: Pick<OverviewPageProps, "service" | "running" | "busy" | "onCommand" | "onCopyAccessCode">) {
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
        <button className="button secondary" type="button" onClick={() => void onCopyAccessCode()}>{text.copyCode}</button>
        <button className="button ghost" type="button" disabled={busy} onClick={() => void onCommand("rotate_access_code")}>{text.rotateCode}</button>
      </div>
    </section>
  );
}

export function OverviewPage({
  service,
  running,
  busy,
  activeTransfers,
  enabledShares,
  draft,
  snapshot,
  selectedNetwork,
  onCommand,
  onCopyAccessCode,
  onNavigate,
}: OverviewPageProps) {
  return (
    <>
      <PageHeading eyebrow={text.commandCenter} title={text.overview} description={text.overviewDescription} />
      <ConnectionPanel service={service} running={running} busy={busy} onCommand={onCommand} onCopyAccessCode={onCopyAccessCode} />
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
                  <button className="button secondary small" type="button" disabled={busy} onClick={() => void onCommand("revoke_session", { sessionId: session.id })}>{text.disconnect}</button>
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
            <button className="text-button" type="button" onClick={() => onNavigate("shares")}>{text.openSection}</button>
          </div>
          <ul>
            <li><span>{text.downloadTitle}</span><b>{draft.downloadShare.enabled ? text.active : text.off}</b></li>
            <li><span>{text.uploadTitle}</span><b>{draft.uploadShare.enabled ? text.active : text.off}</b></li>
          </ul>
        </section>
        <section className="summary-panel">
          <div className="summary-heading">
            <div><h2>{text.networkSecurity}</h2><span>{snapshot.firewall.configured ? text.firewallReady : text.firewallMissingShort}</span></div>
            <button className="text-button" type="button" onClick={() => onNavigate("security")}>{text.openSection}</button>
          </div>
          <p>{selectedNetwork?.name ?? text.automatic} · {selectedNetwork?.address ?? text.noNetworkShort} · {text.portSummary(draft.port)}</p>
        </section>
      </div>
    </>
  );
}

type SharesPageProps = {
  running: boolean;
  busy: boolean;
  noEnabledShares: boolean;
  shareOverlapError?: string | null;
  shareValidationPending: boolean;
  draft: AppSettings;
  downloadShareError?: string | null;
  uploadShareError?: string | null;
  dirty: boolean;
  recoveryPending: boolean;
  saveAvailable: boolean;
  persistBlocked: boolean;
  onUpdate: (patch: Partial<AppSettings>) => void;
  onSave: () => Promise<void>;
};

export function SharesPage(props: SharesPageProps) {
  const {
    running, busy, noEnabledShares, shareOverlapError, shareValidationPending,
    draft, downloadShareError, uploadShareError, dirty, recoveryPending,
    saveAvailable, persistBlocked, onUpdate, onSave,
  } = props;
  return (
    <>
      <PageHeading eyebrow={text.configuration} title={text.shares} description={text.sharesDescription} />
      {running && <div className="locked-notice"><strong>{text.viewOnlyWhileRunning}</strong><span>{text.sharesRuntimeExplanation}</span></div>}
      {!running && noEnabledShares && <div className="notice error" role="alert">{text.oneShareRequired}</div>}
      {!running && shareOverlapError && <div className="notice error" role="alert">{shareOverlapError}</div>}
      {!running && shareValidationPending && <div className="validation-pending" role="status">{text.validatingShares}</div>}
      <div className="share-page-grid">
        <ShareEditor title={text.downloadTitle} description={text.downloadDescription} value={draft.downloadShare} locked={running || busy} error={downloadShareError} onChange={(downloadShare) => onUpdate({ downloadShare })} />
        <ShareEditor title={text.uploadTitle} description={text.uploadDescription} value={draft.uploadShare} locked={running || busy} error={uploadShareError} onChange={(uploadShare) => onUpdate({ uploadShare })} />
      </div>
      <footer className="page-actions">
        <span>{running ? text.stopToEdit : dirty ? text.unsavedChanges : recoveryPending ? text.safeDefaultsPending : text.allChangesSaved}</span>
        <button className="button primary" type="button" disabled={busy || running || !saveAvailable || persistBlocked} onClick={() => void onSave()}>{text.saveShares}</button>
      </footer>
    </>
  );
}

export function TransfersPage({ running, activeTransfers, transferHistory }: {
  running: boolean;
  activeTransfers: TransferInfo[];
  transferHistory: TransferInfo[];
}) {
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

type SecurityPageProps = {
  running: boolean;
  busy: boolean;
  draft: AppSettings;
  snapshot: AppSnapshot;
  selectedNetwork?: NetworkInterfaceInfo;
  draftErrors: DraftValidationErrors;
  persistBlocked: boolean;
  dirty: boolean;
  recoveryPending: boolean;
  saveAvailable: boolean;
  onUpdate: (patch: Partial<AppSettings>) => void;
  onConfigureFirewall: () => Promise<void>;
  onSave: () => Promise<void>;
};

export function SecurityPage(props: SecurityPageProps) {
  const {
    running, busy, draft, snapshot, selectedNetwork, draftErrors, persistBlocked,
    dirty, recoveryPending, saveAvailable, onUpdate, onConfigureFirewall, onSave,
  } = props;
  return (
    <>
      <PageHeading eyebrow={text.configuration} title={text.networkSecurity} description={text.networkDescription} />
      {running && <div className="locked-notice"><strong>{text.viewOnlyWhileRunning}</strong><span>{text.networkRuntimeExplanation}</span></div>}
      <section className="settings-section">
        <div className="section-title-row simple"><h2>{text.networkSettings}</h2></div>
        <div className="settings-grid">
          <label>
            <span>{text.networkInterface}</span>
            <select disabled={running || busy} value={draft.preferredAdapterId ?? ""} onChange={(event) => onUpdate({ preferredAdapterId: event.target.value || null })}>
              <option value="">{text.automatic}</option>
              {snapshot.networks.map((network) => <option value={network.id} key={network.id}>{network.profileName} · {network.name} · {network.address}</option>)}
            </select>
            <small>{running ? text.restartFieldLocked : <NetworkLabel network={selectedNetwork} />}</small>
          </label>
          <label>
            <span>{text.tcpPort}</span>
            <input disabled={running || busy} type="number" min={1024} max={65535} value={draft.port} aria-invalid={Boolean(draftErrors.port)} aria-describedby="port-help" onChange={(event) => onUpdate({ port: Number(event.target.value) })} />
            <small id="port-help" className={draftErrors.port ? "field-error" : undefined} role={draftErrors.port ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.port ?? text.defaultPort}</small>
          </label>
          <label>
            <span>{text.uploadLimit}</span>
            <select disabled={running || busy} value={draft.maxUploadBytes === null ? "unlimited" : String(draft.maxUploadBytes / 1024 ** 3)} aria-invalid={Boolean(draftErrors.maxUploadBytes)} aria-describedby="upload-limit-help" onChange={(event) => onUpdate({ maxUploadBytes: event.target.value === "unlimited" ? null : Number(event.target.value) * 1024 ** 3 })}>
              <option value="5">5 GiB</option><option value="20">20 GiB</option><option value="50">50 GiB</option><option value="100">100 GiB</option><option value="unlimited">{text.unlimited}</option>
            </select>
            <small id="upload-limit-help" className={draftErrors.maxUploadBytes ? "field-error" : undefined} role={draftErrors.maxUploadBytes ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.maxUploadBytes ?? text.diskReserve}</small>
          </label>
          <label>
            <span>{text.inboxStorageLimit}</span>
            <select disabled={running || busy} value={String(draft.maxInboxBytes / 1024 ** 3)} aria-invalid={Boolean(draftErrors.maxInboxBytes)} aria-describedby="inbox-limit-help" onChange={(event) => onUpdate({ maxInboxBytes: Number(event.target.value) * 1024 ** 3 })}>
              <option value="25">25 GiB</option><option value="50">50 GiB</option><option value="100">100 GiB</option><option value="250">250 GiB</option>
            </select>
            <small id="inbox-limit-help" className={draftErrors.maxInboxBytes ? "field-error" : undefined} role={draftErrors.maxInboxBytes ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.maxInboxBytes ?? text.inboxStorageHint}</small>
          </label>
          <label>
            <span>{text.inboxFileLimit}</span>
            <select disabled={running || busy} value={String(draft.maxInboxFiles)} aria-invalid={Boolean(draftErrors.maxInboxFiles)} aria-describedby="inbox-files-help" onChange={(event) => onUpdate({ maxInboxFiles: Number(event.target.value) })}>
              <option value="1000">1.000</option><option value="5000">5.000</option><option value="10000">10.000</option><option value="50000">50.000</option>
            </select>
            <small id="inbox-files-help" className={draftErrors.maxInboxFiles ? "field-error" : undefined} role={draftErrors.maxInboxFiles ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.maxInboxFiles ?? text.inboxFileHint}</small>
          </label>
          <label>
            <span>{text.automaticStop}</span>
            <select disabled={running || busy} value={draft.idleTimeoutMinutes ?? 0} onChange={(event) => onUpdate({ idleTimeoutMinutes: Number(event.target.value) || null })}>
              <option value={0}>{text.off}</option><option value={30}>{text.after30}</option><option value={60}>{text.after60}</option><option value={240}>{text.after240}</option><option value={720}>{text.after720}</option>
            </select>
            <small>{running ? text.restartFieldLocked : text.activeNeverStops}</small>
          </label>
        </div>
      </section>
      <section className="firewall-section">
        <div>
          <p className="eyebrow">{text.windows}</p><h2>{text.windowsFirewall}</h2><p>{snapshot.firewall.detail}</p>
          {running && <span className="field-note">{text.firewallLocked}</span>}
        </div>
        <button className="button secondary" type="button" disabled={busy || running || persistBlocked} onClick={() => void onConfigureFirewall()}>{snapshot.firewall.configured ? text.updateRule : text.setupFirewall}</button>
      </section>
      <footer className="page-actions">
        <span>{running ? text.stopToEdit : dirty ? text.unsavedChanges : recoveryPending ? text.safeDefaultsPending : text.allChangesSaved}</span>
        <button className="button primary" type="button" disabled={busy || running || !saveAvailable || persistBlocked} onClick={() => void onSave()}>{text.saveSettings}</button>
      </footer>
    </>
  );
}

export function DiagnosticsPage({ statusLabel, selectedNetwork, snapshot, busy, onExport }: {
  statusLabel: string;
  selectedNetwork?: NetworkInterfaceInfo;
  snapshot: AppSnapshot;
  busy: boolean;
  onExport: () => Promise<void>;
}) {
  return (
    <>
      <PageHeading eyebrow={text.system} title={text.diagnostics} description={text.diagnosticsDescription} />
      <section className="diagnostics-panel">
        <div><p className="eyebrow">{text.privacy}</p><h2>{text.diagnosticExportTitle}</h2><p>{text.diagnosticPrivacy}</p></div>
        <dl>
          <div><dt>{text.service}</dt><dd>{statusLabel}</dd></div>
          <div><dt>{text.network}</dt><dd>{selectedNetwork ? `${selectedNetwork.name} · ${selectedNetwork.address}` : text.noNetworkShort}</dd></div>
          <div><dt>{text.firewall}</dt><dd>{snapshot.firewall.configured ? text.configured : text.notConfigured}</dd></div>
          <div><dt>{text.version}</dt><dd>{snapshot.appVersion}</dd></div>
        </dl>
        <button className="button primary" type="button" disabled={busy} onClick={() => void onExport()}>{text.exportDiagnostics}</button>
      </section>
    </>
  );
}
