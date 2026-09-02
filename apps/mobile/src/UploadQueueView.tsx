import type { SessionResponse } from "@dmdc/shared";
import { text } from "./i18n";
import { formatBytes } from "./presentation";
import type { UploadItem } from "./uploadQueue";

type UploadQueueViewProps = {
  session: SessionResponse;
  uploads: UploadItem[];
  onFiles: (files: File[]) => void;
  onCancel: (item: UploadItem) => Promise<void>;
  onPause: (item: UploadItem) => void;
  onResume: (item: UploadItem) => void;
  onRetry: (item: UploadItem) => void;
};

export function UploadQueueView({
  session,
  uploads,
  onFiles,
  onCancel,
  onPause,
  onResume,
  onRetry,
}: UploadQueueViewProps) {
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
            {(item.state === "queued" || item.state === "uploading") && (
              <div className="upload-actions">
                <button onClick={() => onPause(item)}>{text.pause}</button>
                <button onClick={() => void onCancel(item)}>{text.cancel}</button>
              </div>
            )}
            {item.state === "paused" && (
              <div className="upload-actions">
                <button onClick={() => onResume(item)}>{text.resume}</button>
                <button onClick={() => void onCancel(item)}>{text.cancel}</button>
              </div>
            )}
            {item.state === "failed" && <button className="retry-button" onClick={() => onRetry(item)}>{text.retry}</button>}
          </article>
        ))}
        {!uploads.length && <p className="empty">{text.noFiles}</p>}
      </div>
    </section>
  );
}
