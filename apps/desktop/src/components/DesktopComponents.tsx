import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { NetworkInterfaceInfo, ShareSettings, TransferInfo } from "@ldtg/shared";
import { text } from "../i18n";
import {
  estimateTransferTiming,
  formatBytes,
  formatDateTime,
  formatDuration,
  formatRate,
} from "../presentation";

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

function TransferTiming({ transfer, compact }: { transfer: TransferInfo; compact: boolean }) {
  const active = transfer.state === "active";
  const [now, setNow] = useState(() => Date.parse(transfer.updatedAt));
  useEffect(() => {
    if (!active) return undefined;
    const timer = globalThis.setInterval(() => setNow(Date.now()), 1_000);
    return () => globalThis.clearInterval(timer);
  }, [active]);
  const timing = estimateTransferTiming({
    startedAt: transfer.startedAt,
    lastProgressAt: transfer.lastProgressAt,
    finishedAt: transfer.finishedAt,
    active,
    transferredBytes: transfer.transferredBytes,
    totalBytes: transfer.totalBytes,
    bytesPerSecond: transfer.bytesPerSecond,
    speedSampleCount: transfer.speedSampleCount,
  }, now);
  const eta = timing.etaQuality === "stable" && timing.remainingSeconds !== null
    ? text.eta(formatDuration(timing.remainingSeconds))
    : timing.etaQuality === "unstable" && timing.remainingSeconds !== null
      ? text.etaUnstable(formatDuration(timing.remainingSeconds))
      : text.etaPending;

  return (
    <div className="transfer-timing">
      {timing.durationSeconds !== null && <span>{text.duration(formatDuration(timing.durationSeconds))}</span>}
      {active && <span>{transfer.bytesPerSecond ? text.speed(formatRate(transfer.bytesPerSecond)) : text.speedPending}</span>}
      {active && <span>{eta}</span>}
      {!compact && <span>{text.startedAt(formatDateTime(transfer.startedAt))}</span>}
      {!compact && transfer.lastProgressAt && <span>{text.lastProgressAt(formatDateTime(transfer.lastProgressAt))}</span>}
    </div>
  );
}

export function TransferRow({ transfer, compact = false }: { transfer: TransferInfo; compact?: boolean }) {
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
      {(active || !compact) && <TransferTiming transfer={transfer} compact={compact} />}
      {!active && !compact && transfer.finishedAt && (
        <time dateTime={transfer.finishedAt}>{text.finishedAt(formatDateTime(transfer.finishedAt))}</time>
      )}
    </article>
  );
}

export function EmptyState({ title, description }: { title: string; description: string }) {
  return (
    <div className="empty-state">
      <strong>{title}</strong>
      <p>{description}</p>
    </div>
  );
}

export function PageHeading({ eyebrow, title, description }: { eyebrow: string; title: string; description: string }) {
  return (
    <header className="page-heading">
      <p className="eyebrow">{eyebrow}</p>
      <h1>{title}</h1>
      <p>{description}</p>
    </header>
  );
}

export function NetworkLabel({ network }: { network?: NetworkInterfaceInfo }) {
  if (!network) return <span>{text.noNetwork}</span>;
  return <span>{network.profileName} · {network.name} · {network.address}/{network.prefixLength} · {network.category}</span>;
}

type ShareEditorProps = {
  title: string;
  description: string;
  value: ShareSettings;
  locked: boolean;
  error?: string | null;
  onChange: (value: ShareSettings) => void;
};

export function ShareEditor({ title, description, value, locked, error, onChange }: ShareEditorProps) {
  async function choose() {
    const selected = await open({ directory: true, multiple: false, title });
    if (typeof selected === "string") onChange({ enabled: true, path: selected });
  }

  return (
    <section className={`share-editor${error ? " invalid" : ""}`}>
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
      {error && <p className="field-error" role="alert">{error}</p>}
      {locked && <p className="field-note">{text.shareLocked}</p>}
    </section>
  );
}
