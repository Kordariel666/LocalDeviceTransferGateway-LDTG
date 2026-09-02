import { FormEvent, useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import type { ApiError, DirectoryResponse, SessionResponse, UploadCreated } from "@dmdc/shared";
import { text } from "./i18n";
import {
  abortableDelay,
  initialUploadQueue,
  nextQueuedUploadId,
  uploadQueueItems,
  uploadQueueReducer,
  type UploadItem,
  type UploadQueueAction,
} from "./uploadQueue";

type View = "download" | "upload";

const brandIconUrl = new URL("../../../assets/icon.svg", import.meta.url).href;

class HttpError extends Error {
  constructor(public status: number, public code: string, message: string) { super(message); }
}

class UploadInterrupted extends Error {}

async function api<T>(url: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(url, { credentials: "same-origin", ...init });
  if (!response.ok) {
    let body: ApiError = { code: `HTTP_${response.status}`, message: `HTTP ${response.status}` };
    try { body = await response.json() as ApiError; } catch { /* Textantwort ignorieren. */ }
    throw new HttpError(response.status, body.code, body.message);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

function formatBytes(value: number): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  if (!value) return "0 B";
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${new Intl.NumberFormat("de-DE", { maximumFractionDigits: 1 }).format(value / 1024 ** index)} ${units[index]}`;
}

function localQueueId(): string {
  const values = new Uint32Array(4);
  globalThis.crypto.getRandomValues(values);
  return Array.from(values, (value) => value.toString(16).padStart(8, "0")).join("");
}

export function App() {
  const [session, setSession] = useState<SessionResponse | null>(null);
  const [checking, setChecking] = useState(true);
  const [loginError, setLoginError] = useState("");
  const [view, setView] = useState<View>("download");
  const [path, setPath] = useState("");
  const [directory, setDirectory] = useState<DirectoryResponse | null>(null);
  const [directoryError, setDirectoryError] = useState("");
  const [search, setSearch] = useState("");
  const [loggingIn, setLoggingIn] = useState(false);
  const loginPending = useRef(false);
  const [uploadQueue, reduceUploadQueue] = useReducer(uploadQueueReducer, initialUploadQueue);
  const uploadQueueRef = useRef(initialUploadQueue);
  const sessionRef = useRef<SessionResponse | null>(null);
  const processingQueue = useRef(false);
  const activeRequests = useRef(new Map<string, XMLHttpRequest>());
  const activeControllers = useRef(new Map<string, AbortController>());
  const retryDelays = useRef(new Map<string, AbortController>());
  const workInterrupts = useRef(new Map<string, AbortController>());
  const pendingCreates = useRef(new Map<string, Promise<UploadCreated>>());
  const directoryRequest = useRef(0);
  const directoryAbort = useRef<AbortController | null>(null);

  const updateQueue = useCallback((action: UploadQueueAction) => {
    uploadQueueRef.current = uploadQueueReducer(uploadQueueRef.current, action);
    reduceUploadQueue(action);
  }, []);

  const uploads = useMemo(() => uploadQueueItems(uploadQueue), [uploadQueue]);

  const handleSessionLost = useCallback(() => {
    sessionRef.current = null;
    setSession(null);
    directoryAbort.current?.abort();
    for (const controller of activeControllers.current.values()) controller.abort();
    for (const controller of retryDelays.current.values()) controller.abort();
    for (const controller of workInterrupts.current.values()) controller.abort();
    for (const request of activeRequests.current.values()) request.abort();
    activeControllers.current.clear();
    retryDelays.current.clear();
    workInterrupts.current.clear();
    activeRequests.current.clear();
    updateQueue({ type: "session-lost", queuedMessage: text.waiting, pausedMessage: text.paused });
  }, [updateQueue]);

  const loadSession = useCallback(async () => {
    try {
      const next = await api<SessionResponse>("/api/v1/session");
      sessionRef.current = next;
      setSession(next);
      setView(next.downloadEnabled ? "download" : "upload");
    } catch (error) {
      if (!(error instanceof HttpError) || error.status !== 401) console.error(error);
      handleSessionLost();
    } finally { setChecking(false); }
  }, [handleSessionLost]);

  useEffect(() => { void loadSession(); }, [loadSession]);

  const loadDirectory = useCallback(async (
    nextPath: string,
    cursor?: string,
    page?: number,
    appliedQuery?: string,
  ) => {
    if (!session?.downloadEnabled) return;
    const request = ++directoryRequest.current;
    directoryAbort.current?.abort();
    const controller = new AbortController();
    directoryAbort.current = controller;
    setDirectoryError("");
    try {
      const query = new URLSearchParams({ path: nextPath });
      if (cursor) query.set("cursor", cursor);
      if (page !== undefined) query.set("page", String(page));
      const requestedQuery = cursor ? (appliedQuery ?? "") : search.trim();
      if (requestedQuery) query.set("q", requestedQuery);
      const value = await api<DirectoryResponse>(`/api/v1/downloads?${query}`, { signal: controller.signal });
      if (request !== directoryRequest.current) return;
      setPath(nextPath);
      setDirectory((current) => cursor && current?.path === value.path && current.query === value.query
        ? { ...value, entries: [...current.entries, ...value.entries] }
        : value);
    } catch (error) {
      if (request !== directoryRequest.current || (error instanceof DOMException && error.name === "AbortError")) return;
      if (error instanceof HttpError && error.status === 401) {
        handleSessionLost();
        return;
      }
      setDirectoryError(error instanceof Error ? error.message : "Ordner nicht erreichbar");
    } finally {
      if (request === directoryRequest.current) directoryAbort.current = null;
    }
  }, [session, search, handleSessionLost]);

  // Navigation calls loadDirectory directly. This effect is only the session/view entry trigger;
  // adding path or search would repeat completed navigation and reload on every search keystroke.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { if (session?.downloadEnabled && view === "download") void loadDirectory(path); }, [session, view]);
  // Queue processing is ref-backed and deliberately starts only when session availability changes.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { if (session?.uploadEnabled) void drainUploadQueue(); }, [session]);

  async function login(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (loginPending.current) return;
    loginPending.current = true;
    setLoggingIn(true);
    setLoginError("");
    const data = new FormData(event.currentTarget);
    try {
      await api("/api/v1/auth", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: data.get("code") }) });
      await loadSession();
    } catch (error) {
      setLoginError(error instanceof Error ? error.message : "Anmeldung fehlgeschlagen");
    } finally {
      loginPending.current = false;
      setLoggingIn(false);
    }
  }

  async function logout() {
    const currentSession = sessionRef.current;
    if (!currentSession) return;
    try {
      await api("/api/v1/logout", {
        method: "POST",
        headers: { "X-DMDC-CSRF": currentSession.csrfToken },
      });
    } catch (error) {
      setLoginError(text.logoutLocal(error instanceof Error ? error.message : "Netzwerkfehler"));
    } finally {
      handleSessionLost();
    }
  }

  function ensureUploadActive(id: string) {
    const state = uploadQueueRef.current.items[id]?.state;
    if (state !== "uploading" && state !== "finalizing") throw new UploadInterrupted();
  }

  async function uploadApi<T>(item: UploadItem, url: string, init: RequestInit = {}): Promise<T> {
    ensureUploadActive(item.id);
    const controller = new AbortController();
    activeControllers.current.set(item.id, controller);
    try {
      const result = await api<T>(url, { ...init, signal: controller.signal });
      ensureUploadActive(item.id);
      return result;
    } finally {
      if (activeControllers.current.get(item.id) === controller) {
        activeControllers.current.delete(item.id);
      }
    }
  }

  function abortUploadWork(id: string) {
    workInterrupts.current.get(id)?.abort();
    activeControllers.current.get(id)?.abort();
    retryDelays.current.get(id)?.abort();
    activeRequests.current.get(id)?.abort();
  }

  async function waitForRetry(id: string, milliseconds: number) {
    ensureUploadActive(id);
    const controller = new AbortController();
    retryDelays.current.set(id, controller);
    try {
      await abortableDelay(milliseconds, controller.signal);
      ensureUploadActive(id);
    } finally {
      if (retryDelays.current.get(id) === controller) retryDelays.current.delete(id);
    }
  }

  function parseXhrError(xhr: XMLHttpRequest): HttpError {
    let body: ApiError = {
      code: `HTTP_${xhr.status}`,
      message: xhr.responseText || "Uploadblock abgelehnt",
    };
    try {
      const parsed = JSON.parse(xhr.responseText) as Partial<ApiError>;
      if (typeof parsed.code === "string" && typeof parsed.message === "string") {
        body = { code: parsed.code, message: parsed.message };
      }
    } catch { /* Nicht strukturierte Antwort behält den HTTP-Fallback. */ }
    return new HttpError(xhr.status, body.code, body.message);
  }

  function isRetryable(error: unknown): boolean {
    if (!(error instanceof HttpError)) return true;
    return error.status === 408 || error.status === 409 || error.status === 425
      || error.status === 429 || error.status >= 500;
  }

  function sendChunk(item: UploadItem, uploadId: string, offset: number, chunk: Blob): Promise<number> {
    return new Promise((resolve, reject) => {
      const currentSession = sessionRef.current;
      if (!currentSession) {
        reject(new HttpError(401, "SESSION_INVALID", "Sitzung ist nicht mehr gültig"));
        return;
      }
      const xhr = new XMLHttpRequest();
      activeRequests.current.set(item.id, xhr);
      xhr.open("PATCH", `/api/v1/uploads/${encodeURIComponent(uploadId)}`);
      xhr.withCredentials = true;
      xhr.setRequestHeader("Content-Type", "application/offset+octet-stream");
      xhr.setRequestHeader("Upload-Offset", String(offset));
      xhr.setRequestHeader("X-DMDC-CSRF", currentSession.csrfToken);
      xhr.upload.onprogress = (event) => {
        const transferred = offset + event.loaded;
        updateQueue({
          type: "progress",
          id: item.id,
          progress: Math.min(99, transferred / item.file.size * 100),
          message: text.transferred(formatBytes(transferred), formatBytes(item.file.size), formatBytes(Math.max(0, item.file.size - transferred))),
        });
      };
      xhr.onload = () => {
        activeRequests.current.delete(item.id);
        if (xhr.status === 200) resolve(Number(xhr.getResponseHeader("Upload-Offset") ?? offset + chunk.size));
        else reject(parseXhrError(xhr));
      };
      xhr.onerror = () => { activeRequests.current.delete(item.id); reject(new Error("Netzwerkverbindung unterbrochen")); };
      xhr.onabort = () => { activeRequests.current.delete(item.id); reject(new Error("Übertragung abgebrochen")); };
      xhr.send(chunk);
    });
  }

  async function getOrCreateUpload(item: UploadItem): Promise<UploadCreated> {
    if (item.uploadId) {
      try {
        return await uploadApi<UploadCreated>(item, `/api/v1/uploads/${encodeURIComponent(item.uploadId)}`);
      } catch (error) {
        if (!(error instanceof HttpError) || error.status !== 404) throw error;
        updateQueue({ type: "set-upload-id", id: item.id, uploadId: undefined, resetProgress: true });
      }
    }
    let creation = pendingCreates.current.get(item.id);
    if (!creation) {
      creation = api<UploadCreated>("/api/v1/uploads", {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-DMDC-CSRF": sessionRef.current?.csrfToken ?? "" },
        body: JSON.stringify({ name: item.file.name, size: item.file.size, lastModified: item.file.lastModified, clientToken: item.id }),
      }).then((created) => {
        if (sessionRef.current?.serviceId === created.serviceId) {
          updateQueue({ type: "set-upload-id", id: item.id, uploadId: created.uploadId });
        }
        if (uploadQueueRef.current.items[item.id]?.state === "cancelled") {
          void removeServerUpload(created.uploadId);
        }
        return created;
      }).finally(() => {
        pendingCreates.current.delete(item.id);
      });
      pendingCreates.current.set(item.id, creation);
    }
    const interrupt = workInterrupts.current.get(item.id);
    if (!interrupt) return creation;
    return new Promise<UploadCreated>((resolve, reject) => {
      const interrupted = () => reject(new UploadInterrupted());
      interrupt.signal.addEventListener("abort", interrupted, { once: true });
      creation.then(resolve, reject).finally(() => {
        interrupt.signal.removeEventListener("abort", interrupted);
      });
    });
  }

  async function removeServerUpload(uploadId: string) {
    const currentSession = sessionRef.current;
    if (!currentSession) return;
    try {
      await api(`/api/v1/uploads/${encodeURIComponent(uploadId)}`, {
        method: "DELETE",
        headers: { "X-DMDC-CSRF": currentSession.csrfToken },
      });
    } catch { /* Best effort */ }
  }

  async function processUpload(item: UploadItem) {
    if (!sessionRef.current) return;
    const interrupt = new AbortController();
    workInterrupts.current.set(item.id, interrupt);
    try {
      const created = await getOrCreateUpload(item);
      updateQueue({ type: "set-upload-id", id: item.id, uploadId: created.uploadId });
      if (uploadQueueRef.current.items[item.id]?.state === "queued" && sessionRef.current) {
        updateQueue({ type: "claim", id: item.id, message: text.preparing });
      }
      ensureUploadActive(item.id);
      let offset = created.offset;
      while (offset < item.file.size) {
        ensureUploadActive(item.id);
        const chunk = item.file.slice(offset, Math.min(item.file.size, offset + created.chunkSize));
        let lastError: unknown;
        for (let attempt = 0; attempt < 3; attempt++) {
          ensureUploadActive(item.id);
          try {
            offset = await sendChunk(item, created.uploadId, offset, chunk);
            ensureUploadActive(item.id);
            lastError = null;
            break;
          }
          catch (error) {
            if (error instanceof HttpError && error.status === 401) throw error;
            if (!isRetryable(error)) throw error;
            lastError = error;
            await waitForRetry(item.id, 800 * 2 ** attempt);
          }
        }
        if (lastError) {
          const status = await uploadApi<UploadCreated>(item, `/api/v1/uploads/${encodeURIComponent(created.uploadId)}`);
          ensureUploadActive(item.id);
          if (status.offset === offset) throw lastError;
          offset = status.offset;
        }
      }
      ensureUploadActive(item.id);
      updateQueue({ type: "finalize", id: item.id, message: text.finalizing });
      let result: { name: string } | undefined;
      let completionError: unknown;
      for (let attempt = 0; attempt < 3; attempt++) {
        const currentSession = sessionRef.current;
        if (!currentSession) throw new HttpError(401, "SESSION_INVALID", "Sitzung ist nicht mehr gültig");
        try {
          result = await uploadApi<{ name: string }>(item, `/api/v1/uploads/${encodeURIComponent(created.uploadId)}/complete`, {
            method: "POST", headers: { "X-DMDC-CSRF": currentSession.csrfToken },
          });
          completionError = undefined;
          break;
        } catch (error) {
          if (error instanceof HttpError && error.status === 401) throw error;
          if (!isRetryable(error)) throw error;
          completionError = error;
          await waitForRetry(item.id, 500 * 2 ** attempt);
        }
      }
      if (!result) throw completionError ?? new Error("Abschlussbestätigung fehlt");
      updateQueue({ type: "complete", id: item.id, message: text.savedAs(result.name) });
    } catch (error) {
      if (error instanceof HttpError && error.status === 401) handleSessionLost();
      updateQueue({
        type: "fail",
        id: item.id,
        message: error instanceof Error ? error.message : "Übertragung fehlgeschlagen",
        errorCode: error instanceof HttpError ? error.code : undefined,
      });
    } finally {
      if (workInterrupts.current.get(item.id) === interrupt) workInterrupts.current.delete(item.id);
    }
  }

  async function drainUploadQueue() {
    if (processingQueue.current) return;
    processingQueue.current = true;
    try {
      while (sessionRef.current) {
        const id = nextQueuedUploadId(uploadQueueRef.current);
        if (!id) break;
        updateQueue({ type: "claim", id, message: text.preparing });
        const item = uploadQueueRef.current.items[id];
        if (!item || item.state !== "uploading") continue;
        await processUpload(item);
      }
    } finally {
      processingQueue.current = false;
      if (sessionRef.current && nextQueuedUploadId(uploadQueueRef.current)) {
        queueMicrotask(() => void drainUploadQueue());
      }
    }
  }

  function queueFiles(files: File[]) {
    if (!files.length) return;
    const items = files.map((file) => ({ id: localQueueId(), file, state: "queued" as const, progress: 0, message: text.waiting }));
    updateQueue({ type: "enqueue", items });
    void drainUploadQueue();
  }

  async function cancelUpload(item: UploadItem) {
    const current = uploadQueueRef.current.items[item.id] ?? item;
    if (current.state === "finalizing" || current.state === "complete") return;
    updateQueue({ type: "cancel", id: item.id, message: text.cancelled });
    abortUploadWork(item.id);
    if (current.uploadId && sessionRef.current) {
      await removeServerUpload(current.uploadId);
    }
  }

  function pauseUpload(item: UploadItem) {
    const current = uploadQueueRef.current.items[item.id] ?? item;
    if (current.state !== "queued" && current.state !== "uploading") return;
    updateQueue({ type: "pause", id: item.id, message: text.paused });
    abortUploadWork(item.id);
  }

  function resumeUpload(item: UploadItem) {
    updateQueue({ type: "resume", id: item.id, message: text.waiting });
    void drainUploadQueue();
  }

  function retryUpload(item: UploadItem) {
    updateQueue({ type: "retry", id: item.id, message: text.waiting });
    void drainUploadQueue();
  }

  const breadcrumbs = useMemo(() => {
    const parts = path.split("/").filter(Boolean);
    return [{ name: text.shareRoot, path: "" }, ...parts.map((name, index) => ({ name, path: parts.slice(0, index + 1).join("/") }))];
  }, [path]);

  if (checking) return <main className="center-card"><div className="spinner" /><p>{text.checking}</p></main>;
  if (!session) return (
    <main className="login-shell">
      <section className="login-card">
        <img className="mobile-brand" src={brandIconUrl} alt="" />
        <p className="eyebrow">{text.title}</p>
        <h1>{text.loginTitle}</h1>
        <p>{text.loginHint}</p>
        {loginError && <div className="error-box">{loginError}</div>}
        <form onSubmit={login}>
          <label>{text.codeLabel}<input name="code" inputMode="numeric" pattern="[0-9]{8}" maxLength={8} autoComplete="one-time-code" enterKeyHint="go" autoFocus required /></label>
          <button type="submit" className="primary-button" disabled={loggingIn}>{text.connect}</button>
        </form>
        <aside>{text.localWarning}</aside>
      </section>
    </main>
  );

  return (
    <div className="mobile-shell">
      <header>
        <div className="mobile-wordmark">
          <img src={brandIconUrl} alt="" />
          <div><strong>DMDC</strong><span>{text.localDirect}</span></div>
        </div>
        <button type="button" onClick={logout}>{text.disconnect}</button>
      </header>
      <nav aria-label={text.transferNavigation}>
        {session.downloadEnabled && <button type="button" aria-current={view === "download" ? "page" : undefined} className={view === "download" ? "active" : ""} onClick={() => setView("download")}>{text.downloadTab}</button>}
        {session.uploadEnabled && <button type="button" aria-current={view === "upload" ? "page" : undefined} className={view === "upload" ? "active" : ""} onClick={() => setView("upload")}>{text.uploadTab}</button>}
      </nav>
      <main className="mobile-content">
        {view === "download" && session.downloadEnabled && (
          <section>
            <p className="eyebrow">{text.readOnly}</p><h1>{text.filesFromPc}</h1><p className="intro">{text.downloadIntro}</p>
            <form className="search-row" onSubmit={(event) => { event.preventDefault(); void loadDirectory(path); }}>
              <input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={text.searchPlaceholder} aria-label={text.searchPlaceholder} />
              <button type="submit">{text.search}</button>
            </form>
            <div className="breadcrumbs" aria-label={text.currentPath}>{breadcrumbs.map((item) => <button type="button" key={item.path} onClick={() => loadDirectory(item.path)}><bdi className="untrusted-name">{item.name}</bdi></button>)}</div>
            {directoryError && <div className="error-box">{directoryError}</div>}
            <div className="file-list">
              {directory?.entries.map((entry) => entry.kind === "directory" ? (
                <button className="file-row folder-row" type="button" key={entry.path} onClick={() => loadDirectory(entry.path)}><span><strong><bdi className="untrusted-name">{entry.name}</bdi></strong><small>{text.folder}</small></span><b>{text.openFolder}</b></button>
              ) : (
                <a className="file-row file-download-row" key={entry.path} href={`/api/v1/download?path=${encodeURIComponent(entry.path)}`} download><span><strong><bdi className="untrusted-name">{entry.name}</bdi></strong><small>{formatBytes(entry.size)}{entry.modifiedAt ? ` · ${new Intl.DateTimeFormat("de-DE", { dateStyle: "short", timeStyle: "short" }).format(new Date(entry.modifiedAt))}` : ""}</small></span><b>{text.load}</b></a>
              ))}
              {directory && !directory.entries.length && <p className="empty">{text.emptyFolder}</p>}
              {directory?.nextCursor && directory.nextPage !== null && <button className="more-button" onClick={() => loadDirectory(path, directory.nextCursor ?? undefined, directory.nextPage ?? undefined, directory.query)}>{text.moreFiles}</button>}
            </div>
          </section>
        )}
        {view === "upload" && session.uploadEnabled && (
          <section>
            <p className="eyebrow">{text.addOnly}</p><h1>{text.filesToPc}</h1><p className="intro">{text.uploadIntro}</p>
            <label className="file-picker"><input type="file" multiple onChange={(event) => { void queueFiles([...(event.target.files ?? [])]); event.target.value = ""; }} /><strong>{text.chooseFiles}</strong><span>{text.allowedTypes(session.maxUploadBytes ? formatBytes(session.maxUploadBytes) : text.byFreeSpace)}</span></label>
            <div className="upload-list" aria-live="polite">
              {uploads.map((item) => (
                <article className={`upload-item ${item.state}`} data-error-code={item.errorCode} key={item.id}>
                  <header className="upload-heading">
                    <div><strong><bdi className="untrusted-name">{item.file.name}</bdi></strong><span>{formatBytes(item.file.size)} · {item.message}</span></div>
                    <b className="upload-state">{text.uploadState(item.state)}</b>
                  </header>
                  <div className="upload-progress"><progress aria-label={`${item.file.name}: ${Math.round(item.progress)} Prozent`} max={100} value={item.progress} /><span>{Math.round(item.progress)} %</span></div>
                  {(item.state === "queued" || item.state === "uploading") && <div className="upload-actions"><button onClick={() => pauseUpload(item)}>{text.pause}</button><button onClick={() => cancelUpload(item)}>{text.cancel}</button></div>}
                  {item.state === "paused" && <div className="upload-actions"><button onClick={() => resumeUpload(item)}>{text.resume}</button><button onClick={() => cancelUpload(item)}>{text.cancel}</button></div>}
                  {item.state === "failed" && <button className="retry-button" onClick={() => retryUpload(item)}>{text.retry}</button>}
                </article>
              ))}
              {!uploads.length && <p className="empty">{text.noFiles}</p>}
            </div>
          </section>
        )}
      </main>
      <footer>{text.localWarning}</footer>
    </div>
  );
}
