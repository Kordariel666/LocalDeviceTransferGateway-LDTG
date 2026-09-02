import type { SessionResponse } from "@dmdc/shared";
import { text } from "./i18n";
import { formatBytes } from "./presentation";
import type { UploadItem, UploadQueueSummary } from "./uploadQueue";

type UploadQueueViewProps = {
  session: SessionResponse;
  uploads: UploadItem[];
  summary: UploadQueueSummary;
  sessionNotice: string | null;
  onFiles: (files: File[]) => void;
  onCancel: (item: UploadItem) => Promise<void>;
  onPause: (item: UploadItem) => void;
  onResume: (item: UploadItem) => void;
  onRetry: (item: UploadItem) => void;
  onPauseAll: () => void;
  onResumeAll: () => void;
  onRetryFailed: () => void;
  onClearFinished: () => void;
  onRemoveQueued: (item: UploadItem) => void;
  onDismissSessionNotice: () => void;
};

export function UploadQueueView({
  session,
  uploads,
  summary,
  sessionNotice,
  onFiles,
  onCancel,
  onPause,
  onResume,
  onRetry,
  onPauseAll,
  onResumeAll,
  onRetryFailed,
  onClearFinished,
  onRemoveQueued,
  onDismissSessionNotice,
}: UploadQueueViewProps) {
  const hasPausable = uploads.some((item) => item.state === "queued" || item.state === "uploading");
  const hasPaused = uploads.some((item) => item.state === "paused");
  const hasFailed = uploads.some((item) => item.state === "failed");
  const hasFinished = uploads.some((item) => item.state === "complete" || item.state === "cancelled");

  return (
    <section>
      <p className="eyebrow">{text.addOnly}</p>
      <h1>{text.filesToPc}</h1>
      <p className="intro">{text.uploadIntro}</p>
      <label className="file-picker">
        <input
          type="file"
          multiple
          onChange={(event) => {
            onFiles([...(event.target.files ?? [])]);
            event.target.value = "";
          }}
        />
        <strong>{text.chooseFiles}</strong>
        <span>{text.allowedTypes(session.maxUploadBytes ? formatBytes(session.maxUploadBytes) : text.byFreeSpace)}</span>
      </label>
      {sessionNotice && uploads.length > 0 && (
        <aside className="upload-session-notice" role="status">
          <span>{sessionNotice}</span>
          <button type="button" onClick={onDismissSessionNotice}>{text.dismiss}</button>
        </aside>
      )}
      {uploads.length > 0 && (
        <section className="upload-batch" aria-label={text.batchSummaryLabel}>
          <header>
            <div>
              <strong>{text.batchProgress}</strong>
              <span>{text.finishedFiles(summary.finishedFiles, summary.totalFiles)} · {formatBytes(summary.transferredBytes)} von {formatBytes(summary.totalBytes)}</span>
            </div>
            <b>{Math.round(summary.progress)} %</b>
          </header>
          <progress aria-label={text.batchProgressLabel} max={100} value={summary.progress} />
          <div className="upload-batch-actions">
            <button type="button" disabled={!hasPausable} onClick={onPauseAll}>{text.pauseAll}</button>
            <button type="button" disabled={!hasPaused} onClick={onResumeAll}>{text.resumeAll}</button>
            <button type="button" disabled={!hasFailed} onClick={onRetryFailed}>{text.retryFailed}</button>
            <button type="button" disabled={!hasFinished} onClick={onClearFinished}>{text.clearFinished}</button>
          </div>
        </section>
      )}
      <div className="upload-list" aria-live="polite">
        {uploads.map((item) => (
          <article className={`upload-item ${item.state}`} data-error-code={item.errorCode} key={item.id}>
            <header className="upload-heading">
              <div>
                <strong><bdi className="untrusted-name">{item.file.name}</bdi></strong>
                <span>{formatBytes(item.file.size)} · {item.message}</span>
              </div>
              <b className="upload-state">{text.uploadState(item.state)}</b>
            </header>
            <div className="upload-progress">
              <progress aria-label={`${item.file.name}: ${Math.round(item.progress)} Prozent`} max={100} value={item.progress} />
              <span>{Math.round(item.progress)} %</span>
            </div>
            {item.state === "queued" && (
              <div className="upload-actions">
                <button type="button" onClick={() => onPause(item)}>{text.pause}</button>
                <button type="button" onClick={() => onRemoveQueued(item)}>{text.remove}</button>
              </div>
            )}
            {item.state === "uploading" && (
              <div className="upload-actions">
                <button type="button" onClick={() => onPause(item)}>{text.pause}</button>
                <button type="button" onClick={() => void onCancel(item)}>{text.cancel}</button>
              </div>
            )}
            {item.state === "paused" && (
              <div className="upload-actions">
                <button type="button" onClick={() => onResume(item)}>{text.resume}</button>
                <button type="button" onClick={() => void onCancel(item)}>{text.cancel}</button>
              </div>
            )}
            {item.state === "failed" && <button type="button" className="retry-button" onClick={() => onRetry(item)}>{text.retry}</button>}
          </article>
        ))}
        {!uploads.length && <p className="empty">{text.noFiles}</p>}
      </div>
    </section>
  );
}
