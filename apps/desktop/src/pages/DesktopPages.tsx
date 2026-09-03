import { useMemo, useState } from "react";
import { QRCodeSVG } from "qrcode.react";
import type {
  AppSettings,
  AppSnapshot,
  LimitSettings,
  NetworkInterfaceInfo,
  ServiceStatus,
  ShareProfile,
  TransferDirection,
  TransferInfo,
  TransferState,
} from "@ldtg/shared";
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
import type { EffectiveProfileSettings } from "../profileSettings";

type Command = (command: string, args?: Record<string, unknown>) => Promise<void>;

type OverviewPageProps = {
  service: ServiceStatus;
  running: boolean;
  busy: boolean;
  activeTransfers: TransferInfo[];
  enabledShares: number;
  profile: ShareProfile;
  effectiveSettings: EffectiveProfileSettings;
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
  profile,
  effectiveSettings,
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
                  <div className="device-details">
                    <div className="device-title">
                      <strong><bdi className="untrusted-name">{session.deviceName ?? session.clientName}</bdi></strong>
                      {session.deviceName && <span><bdi>{session.clientName}</bdi></span>}
                    </div>
                    <dl className="device-meta">
                      <div><dt>{text.ipAddress}</dt><dd><bdi>{session.address}</bdi></dd></div>
                      <div><dt>{text.connectedAt}</dt><dd><time dateTime={session.createdAt}>{formatDateTime(session.createdAt)}</time></dd></div>
                      <div><dt>{text.lastActivity}</dt><dd><time dateTime={session.lastActivity}>{formatDateTime(session.lastActivity)}</time></dd></div>
                      <div><dt>{text.activeTransfersShort}</dt><dd>{text.activeTransferCount(activeTransfers.filter((transfer) => transfer.sessionId === session.id).length)}</dd></div>
                    </dl>
                  </div>
                  <button className="button secondary small" type="button" disabled={busy} aria-label={text.disconnectDevice(session.deviceName ?? session.clientName)} onClick={() => void onCommand("revoke_session", { sessionId: session.id })}>{text.disconnect}</button>
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
            <div><h2>{text.shares}</h2><span>{profile.name} · {text.enabledShareCount(enabledShares)}</span></div>
            <button className="text-button" type="button" onClick={() => onNavigate("shares")}>{text.openSection}</button>
          </div>
          <ul>
            <li><span>{text.downloadTitle}</span><b>{profile.downloadShare.enabled ? text.active : text.off}</b></li>
            <li><span>{text.uploadTitle}</span><b>{profile.uploadShare.enabled ? text.active : text.off}</b></li>
          </ul>
        </section>
        <section className="summary-panel">
          <div className="summary-heading">
            <div><h2>{text.networkSecurity}</h2><span>{snapshot.firewall.configured ? text.firewallReady : text.firewallMissingShort}</span></div>
            <button className="text-button" type="button" onClick={() => onNavigate("security")}>{text.openSection}</button>
          </div>
          <p>{selectedNetwork?.name ?? text.automatic} · {selectedNetwork?.address ?? text.noNetworkShort} · {text.portSummary(effectiveSettings.port)}</p>
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
  profile: ShareProfile;
  downloadShareError?: string | null;
  uploadShareError?: string | null;
  profileNameError?: string | null;
  dirty: boolean;
  recoveryPending: boolean;
  saveAvailable: boolean;
  persistBlocked: boolean;
  onUpdateProfile: (patch: Partial<ShareProfile>) => void;
  onSelectProfile: (profileId: string) => void;
  onDuplicateProfile: () => void;
  onDeleteProfile: () => Promise<void>;
  onSave: () => Promise<void>;
};

export function SharesPage(props: SharesPageProps) {
  const {
    running, busy, noEnabledShares, shareOverlapError, shareValidationPending,
    draft, profile, downloadShareError, uploadShareError, profileNameError, dirty, recoveryPending,
    saveAvailable, persistBlocked, onUpdateProfile, onSelectProfile,
    onDuplicateProfile, onDeleteProfile, onSave,
  } = props;
  return (
    <>
      <PageHeading eyebrow={text.configuration} title={text.shares} description={text.sharesDescription} />
      {running && <div className="locked-notice"><strong>{text.viewOnlyWhileRunning}</strong><span>{text.sharesRuntimeExplanation}</span></div>}
      <section className="profile-bar" aria-label={text.shareProfiles}>
        <label>
          <span>{text.activeProfile}</span>
          <select disabled={running || busy} value={profile.id} onChange={(event) => onSelectProfile(event.target.value)}>
            {draft.profiles.map((entry) => <option key={entry.id} value={entry.id}>{entry.name}</option>)}
          </select>
        </label>
        <label>
          <span>{text.profileName}</span>
          <input
            disabled={running || busy}
            maxLength={64}
            value={profile.name}
            aria-label={text.profileName}
            aria-invalid={Boolean(profileNameError)}
            aria-describedby="profile-name-help"
            onChange={(event) => onUpdateProfile({ name: event.target.value })}
          />
          <small id="profile-name-help" className={profileNameError ? "field-error" : undefined}>{profileNameError ?? text.profileNameHint}</small>
        </label>
        <div className="profile-actions">
          <button className="button secondary small" type="button" disabled={running || busy || Boolean(profileNameError) || draft.profiles.length >= 32} onClick={onDuplicateProfile}>{text.duplicateProfile}</button>
          <button className="button ghost small" type="button" disabled={running || busy || draft.profiles.length <= 1} onClick={() => void onDeleteProfile()}>{text.deleteProfile}</button>
        </div>
      </section>
      {!running && noEnabledShares && <div className="notice error" role="alert">{text.oneShareRequired}</div>}
      {!running && shareOverlapError && <div className="notice error" role="alert">{shareOverlapError}</div>}
      {!running && shareValidationPending && <div className="validation-pending" role="status">{text.validatingShares}</div>}
      <div className="share-page-grid">
        <ShareEditor title={text.downloadTitle} description={text.downloadDescription} value={profile.downloadShare} locked={running || busy} error={downloadShareError} onChange={(downloadShare) => onUpdateProfile({ downloadShare })} />
        <ShareEditor title={text.uploadTitle} description={text.uploadDescription} value={profile.uploadShare} locked={running || busy} error={uploadShareError} onChange={(uploadShare) => onUpdateProfile({ uploadShare })} />
      </div>
      <footer className="page-actions">
        <span>{running ? text.stopToEdit : dirty ? text.unsavedChanges : recoveryPending ? text.safeDefaultsPending : text.allChangesSaved}</span>
        <button className="button primary" type="button" disabled={busy || running || !saveAvailable || persistBlocked} onClick={() => void onSave()}>{text.saveShares}</button>
      </footer>
    </>
  );
}

export function TransfersPage({
  running,
  busy,
  activeTransfers,
  transferHistory,
  stopAfterBatch,
  onStopAfterBatchChange,
  onClearHistory,
}: {
  running: boolean;
  busy: boolean;
  activeTransfers: TransferInfo[];
  transferHistory: TransferInfo[];
  stopAfterBatch: boolean;
  onStopAfterBatchChange: (enabled: boolean) => void;
  onClearHistory: () => Promise<void>;
}) {
  const [directionFilter, setDirectionFilter] = useState<"all" | TransferDirection>("all");
  const [statusFilter, setStatusFilter] = useState<"all" | Exclude<TransferState, "active">>("all");
  const canEnableStopAfterBatch = running && activeTransfers.length > 0;
  const filteredHistory = useMemo(() => transferHistory.filter((transfer) => (
    (directionFilter === "all" || transfer.direction === directionFilter)
    && (statusFilter === "all" || transfer.state === statusFilter)
  )), [directionFilter, statusFilter, transferHistory]);
  const historyCount = filteredHistory.length === transferHistory.length
    ? text.entryCount(transferHistory.length)
    : text.filteredEntryCount(filteredHistory.length, transferHistory.length);
  return (
    <>
      <PageHeading eyebrow={text.activity} title={text.transfers} description={text.transfersDescription} />
      <section className={`runtime-option${stopAfterBatch ? " armed" : ""}`}>
        <div>
          <p className="eyebrow">{text.runtimeOption}</p>
          <h2>{text.stopAfterBatchShort}</h2>
          <p>{canEnableStopAfterBatch || stopAfterBatch ? text.stopAfterBatchHint : text.stopAfterBatchUnavailable}</p>
        </div>
        <label className="switch runtime-switch">
          <input
            type="checkbox"
            checked={stopAfterBatch}
            disabled={busy || (!stopAfterBatch && !canEnableStopAfterBatch)}
            aria-label={text.stopAfterBatch}
            onChange={(event) => onStopAfterBatchChange(event.target.checked)}
          />
          <span aria-hidden="true" />
          <b>{stopAfterBatch ? text.active : text.off}</b>
        </label>
      </section>
      <section className="transfer-section">
        <div className="section-title-row simple"><h2>{text.activeTransfers}</h2><span>{text.transferCount(activeTransfers.length)}</span></div>
        {!activeTransfers.length ? (
          <EmptyState title={text.noActiveTransfers} description={running ? text.noTransfersDescription : text.transfersAfterStart} />
        ) : (
          <div className="transfer-list roomy">{activeTransfers.map((transfer) => <TransferRow transfer={transfer} key={transfer.id} />)}</div>
        )}
      </section>
      <section className="transfer-section history">
        <div className="section-title-row simple history-heading">
          <div>
            <h2>{text.transferHistory}</h2>
            <p>{text.historyScope}</p>
          </div>
          <div className="history-summary">
            <span>{historyCount}</span>
            <button className="button secondary small" type="button" disabled={busy || transferHistory.length === 0} onClick={() => void onClearHistory()}>{text.clearHistory}</button>
          </div>
        </div>
        <div className="history-filters" aria-label={text.transferHistory}>
          <label>
            <span>{text.historyDirectionFilter}</span>
            <select aria-label={text.historyDirectionFilter} value={directionFilter} onChange={(event) => setDirectionFilter(event.target.value as "all" | TransferDirection)}>
              <option value="all">{text.allDirections}</option>
              <option value="upload">{text.fromPhone}</option>
              <option value="download">{text.toPhone}</option>
            </select>
          </label>
          <label>
            <span>{text.historyStatusFilter}</span>
            <select aria-label={text.historyStatusFilter} value={statusFilter} onChange={(event) => setStatusFilter(event.target.value as "all" | Exclude<TransferState, "active">)}>
              <option value="all">{text.allStatuses}</option>
              <option value="complete">{text.transferState("complete")}</option>
              <option value="failed">{text.transferState("failed")}</option>
              <option value="cancelled">{text.transferState("cancelled")}</option>
              <option value="expired">{text.transferState("expired")}</option>
            </select>
          </label>
        </div>
        {!transferHistory.length ? (
          <EmptyState title={text.noHistory} description={text.noHistoryDescription} />
        ) : !filteredHistory.length ? (
          <EmptyState title={text.noMatchingHistory} description={text.noMatchingHistoryDescription} />
        ) : (
          <div className="transfer-list roomy">{filteredHistory.map((transfer) => <TransferRow transfer={transfer} key={transfer.id} />)}</div>
        )}
      </section>
    </>
  );
}

type SecurityPageProps = {
  running: boolean;
  busy: boolean;
  draft: AppSettings;
  profile: ShareProfile;
  effectiveSettings: EffectiveProfileSettings;
  snapshot: AppSnapshot;
  selectedNetwork?: NetworkInterfaceInfo;
  draftErrors: DraftValidationErrors;
  persistBlocked: boolean;
  dirty: boolean;
  recoveryPending: boolean;
  saveAvailable: boolean;
  onUpdateNetwork: (preferredAdapterId: string | null) => void;
  onUpdatePort: (port: number) => void;
  onUpdateLimits: (patch: Partial<LimitSettings>) => void;
  onSetOverride: (kind: "network" | "port" | "limits", enabled: boolean) => void;
  onConfigureFirewall: () => Promise<void>;
  onForgetTrustedNetwork: (networkId: string | null) => Promise<void>;
  onSave: () => Promise<void>;
};

export function SecurityPage(props: SecurityPageProps) {
  const {
    running, busy, draft, profile, effectiveSettings, snapshot, selectedNetwork, draftErrors, persistBlocked,
    dirty, recoveryPending, saveAvailable, onUpdateNetwork, onUpdatePort,
    onUpdateLimits, onSetOverride, onConfigureFirewall,
    onForgetTrustedNetwork, onSave,
  } = props;
  return (
    <>
      <PageHeading eyebrow={text.configuration} title={text.networkSecurity} description={text.networkDescription} />
      {running && <div className="locked-notice"><strong>{text.viewOnlyWhileRunning}</strong><span>{text.networkRuntimeExplanation}</span></div>}
      <section className="settings-section">
        <div className="section-title-row simple"><div><h2>{text.networkSettings}</h2><p>{text.profileSettingsContext(profile.name)}</p></div></div>
        <div className="override-grid" aria-label={text.profileOverrides}>
          <label><input type="checkbox" disabled={running || busy} checked={profile.overrides.network !== null} onChange={(event) => onSetOverride("network", event.target.checked)} /> <span>{text.overrideNetwork}</span></label>
          <label><input type="checkbox" disabled={running || busy} checked={profile.overrides.port !== null} onChange={(event) => onSetOverride("port", event.target.checked)} /> <span>{text.overridePort}</span></label>
          <label><input type="checkbox" disabled={running || busy} checked={profile.overrides.limits !== null} onChange={(event) => onSetOverride("limits", event.target.checked)} /> <span>{text.overrideLimits}</span></label>
        </div>
        <div className="settings-grid">
          <label>
            <span>{text.networkInterface}</span>
            <select disabled={running || busy} value={effectiveSettings.preferredAdapterId ?? ""} onChange={(event) => onUpdateNetwork(event.target.value || null)}>
              <option value="">{text.automatic}</option>
              {snapshot.networks.map((network) => <option value={network.id} key={network.id}>{network.profileName} · {network.name} · {network.address}</option>)}
            </select>
            <small>{running ? text.restartFieldLocked : <NetworkLabel network={selectedNetwork} />}</small>
          </label>
          <label>
            <span>{text.tcpPort}</span>
            <input disabled={running || busy} type="number" min={1024} max={65535} value={effectiveSettings.port} aria-invalid={Boolean(draftErrors.port)} aria-describedby="port-help" onChange={(event) => onUpdatePort(Number(event.target.value))} />
            <small id="port-help" className={draftErrors.port ? "field-error" : undefined} role={draftErrors.port ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.port ?? text.defaultPort}</small>
          </label>
          <label>
            <span>{text.uploadLimit}</span>
            <select disabled={running || busy} value={effectiveSettings.maxUploadBytes === null ? "unlimited" : String(effectiveSettings.maxUploadBytes / 1024 ** 3)} aria-invalid={Boolean(draftErrors.maxUploadBytes)} aria-describedby="upload-limit-help" onChange={(event) => onUpdateLimits({ maxUploadBytes: event.target.value === "unlimited" ? null : Number(event.target.value) * 1024 ** 3 })}>
              <option value="5">5 GiB</option><option value="20">20 GiB</option><option value="50">50 GiB</option><option value="100">100 GiB</option><option value="unlimited">{text.unlimited}</option>
            </select>
            <small id="upload-limit-help" className={draftErrors.maxUploadBytes ? "field-error" : undefined} role={draftErrors.maxUploadBytes ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.maxUploadBytes ?? text.diskReserve}</small>
          </label>
          <label>
            <span>{text.inboxStorageLimit}</span>
            <select disabled={running || busy} value={String(effectiveSettings.maxInboxBytes / 1024 ** 3)} aria-invalid={Boolean(draftErrors.maxInboxBytes)} aria-describedby="inbox-limit-help" onChange={(event) => onUpdateLimits({ maxInboxBytes: Number(event.target.value) * 1024 ** 3 })}>
              <option value="25">25 GiB</option><option value="50">50 GiB</option><option value="100">100 GiB</option><option value="250">250 GiB</option>
            </select>
            <small id="inbox-limit-help" className={draftErrors.maxInboxBytes ? "field-error" : undefined} role={draftErrors.maxInboxBytes ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.maxInboxBytes ?? text.inboxStorageHint}</small>
          </label>
          <label>
            <span>{text.inboxFileLimit}</span>
            <select disabled={running || busy} value={String(effectiveSettings.maxInboxFiles)} aria-invalid={Boolean(draftErrors.maxInboxFiles)} aria-describedby="inbox-files-help" onChange={(event) => onUpdateLimits({ maxInboxFiles: Number(event.target.value) })}>
              <option value="1000">1.000</option><option value="5000">5.000</option><option value="10000">10.000</option><option value="50000">50.000</option>
            </select>
            <small id="inbox-files-help" className={draftErrors.maxInboxFiles ? "field-error" : undefined} role={draftErrors.maxInboxFiles ? "alert" : undefined}>{running ? text.restartFieldLocked : draftErrors.maxInboxFiles ?? text.inboxFileHint}</small>
          </label>
          <label>
            <span>{text.automaticStop}</span>
            <select disabled={running || busy} value={effectiveSettings.idleTimeoutMinutes ?? 0} onChange={(event) => onUpdateLimits({ idleTimeoutMinutes: Number(event.target.value) || null })}>
              <option value={0}>{text.off}</option><option value={30}>{text.after30}</option><option value={60}>{text.after60}</option><option value={240}>{text.after240}</option><option value={720}>{text.after720}</option>
            </select>
            <small>{running ? text.restartFieldLocked : text.activeNeverStops}</small>
          </label>
        </div>
      </section>
      <section className="trusted-networks-section">
        <div className="section-title-row trusted-networks-heading">
          <div>
            <p className="eyebrow">{text.networkSecurity}</p>
            <h2>{text.trustedNetworks}</h2>
            <p>{text.trustedNetworksDescription}</p>
          </div>
          <button className="button secondary small" type="button" disabled={running || busy || draft.trustedNetworks.length === 0} onClick={() => void onForgetTrustedNetwork(null)}>{text.forgetAllNetworks}</button>
        </div>
        {!draft.trustedNetworks.length ? (
          <EmptyState title={text.noTrustedNetworks} description={text.noTrustedNetworksDescription} />
        ) : (
          <div className="trusted-network-list">
            {draft.trustedNetworks.map((trusted) => {
              const current = snapshot.networks.find((network) => (
                network.profileResolved && network.networkId === trusted.id
              ));
              const name = current?.profileName ?? trusted.name;
              const category = current?.category ?? trusted.category;
              return (
                <article className={`trusted-network-row${current ? "" : " stale"}`} key={trusted.id}>
                  <div className="trusted-network-name">
                    <strong><bdi className="untrusted-name">{name}</bdi></strong>
                    <span className={`status-chip${current ? "" : " offline"}`}>{current ? text.networkAvailable : text.networkStale}</span>
                  </div>
                  <dl>
                    <div><dt>{text.networkCategory}</dt><dd><bdi className="untrusted-name">{category}</bdi></dd></div>
                    <div><dt>{text.networkLastUsed}</dt><dd>{trusted.lastUsedAt ? formatDateTime(trusted.lastUsedAt) : text.networkNeverUsed}</dd></div>
                  </dl>
                  <button className="button secondary small" type="button" disabled={running || busy} aria-label={text.forgetNetworkLabel(name)} onClick={() => void onForgetTrustedNetwork(trusted.id)}>{text.forgetNetwork}</button>
                </article>
              );
            })}
          </div>
        )}
        {running && <p className="field-note trusted-networks-lock">{text.trustedNetworksLocked}</p>}
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
