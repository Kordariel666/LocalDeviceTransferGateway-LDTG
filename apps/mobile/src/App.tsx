import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ApiError, DirectoryResponse, SessionResponse, UploadCreated } from "@dmdc/shared";
import { text } from "./i18n";

type View = "download" | "upload";
type UploadState = "queued" | "uploading" | "paused" | "finalizing" | "complete" | "failed" | "cancelled";
type UploadItem = { id: string; file: File; state: UploadState; progress: number; message: string; uploadId?: string };

const brandIconUrl = new URL("../../../assets/icon.svg", import.meta.url).href;

class HttpError extends Error {
  constructor(public status: number, public code: string, message: string) { super(message); }
}

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
  const [uploads, setUploads] = useState<UploadItem[]>([]);
  const sessionRef = useRef<SessionResponse | null>(null);
  const uploadRecords = useRef(new Map<string, UploadItem>());
  const pendingUploads = useRef<string[]>([]);
  const processingQueue = useRef(false);
  const activeRequests = useRef(new Map<string, XMLHttpRequest>());
  const activeControllers = useRef(new Map<string, AbortController>());
  const cancelled = useRef(new Set<string>());
  const paused = useRef(new Set<string>());
  const directoryRequest = useRef(0);
  const directoryAbort = useRef<AbortController | null>(null);

  const handleSessionLost = useCallback(() => {
    sessionRef.current = null;
    setSession(null);
    directoryAbort.current?.abort();
    for (const controller of activeControllers.current.values()) controller.abort();
    for (const request of activeRequests.current.values()) request.abort();
    activeControllers.current.clear();
    activeRequests.current.clear();
    const retry: string[] = [];
    for (const [id, item] of uploadRecords.current) {
      if (item.state === "complete" || item.state === "cancelled") continue;
      const next: UploadItem = paused.current.has(item.id)
        ? { ...item, uploadId: undefined, state: "paused", progress: 0, message: text.paused }
        : { ...item, uploadId: undefined, state: "queued", progress: 0, message: text.waiting };
      uploadRecords.current.set(id, next);
      if (!paused.current.has(id) && !cancelled.current.has(id)) retry.push(id);
    }
    setUploads((current) => current.map((item) => uploadRecords.current.get(item.id) ?? item));
    pendingUploads.current = [
      ...new Set([...retry, ...pendingUploads.current.filter((id) => !cancelled.current.has(id))]),
    ];
  }, []);

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
    setLoginError("");
    const data = new FormData(event.currentTarget);
    try {
      await api("/api/v1/auth", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: data.get("code") }) });
      await loadSession();
    } catch (error) { setLoginError(error instanceof Error ? error.message : "Anmeldung fehlgeschlagen"); }
  }

  async function logout() {
    if (!session) return;
    await api("/api/v1/logout", { method: "POST", headers: { "X-DMDC-CSRF": session.csrfToken } });
    handleSessionLost();
  }

  function updateUpload(id: string, patch: Partial<UploadItem>) {
    const current = uploadRecords.current.get(id);
    if (current) uploadRecords.current.set(id, { ...current, ...patch });
    setUploads((current) => current.map((item) => item.id === id ? { ...item, ...patch } : item));
  }

  function ensureUploadActive(id: string) {
    if (cancelled.current.has(id)) throw new Error(text.cancelled);
    if (paused.current.has(id)) throw new Error(text.paused);
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
    activeControllers.current.get(id)?.abort();
    activeRequests.current.get(id)?.abort();
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
        updateUpload(item.id, { progress: Math.min(99, transferred / item.file.size * 100), message: text.transferred(formatBytes(transferred), formatBytes(item.file.size), formatBytes(Math.max(0, item.file.size - transferred))) });
      };
      xhr.onload = () => {
        activeRequests.current.delete(item.id);
        if (xhr.status === 200) resolve(Number(xhr.getResponseHeader("Upload-Offset") ?? offset + chunk.size));
        else reject(new HttpError(xhr.status, `HTTP_${xhr.status}`, xhr.responseText || "Uploadblock abgelehnt"));
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
        updateUpload(item.id, { uploadId: undefined, progress: 0 });
      }
    }
    // The create response is deliberately not aborted: the server may already have
    // committed the upload ID, which the client must retain before honoring Pause.
    return api<UploadCreated>("/api/v1/uploads", {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-DMDC-CSRF": sessionRef.current?.csrfToken ?? "" },
      body: JSON.stringify({ name: item.file.name, size: item.file.size, lastModified: item.file.lastModified, clientToken: item.id }),
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
    const current = uploadRecords.current.get(item.id) ?? item;
    if (!sessionRef.current || cancelled.current.has(item.id) || current.state === "complete") return;
    item = current;
    ensureUploadActive(item.id);
    updateUpload(item.id, { state: "uploading", message: text.preparing });
    try {
      const created = await getOrCreateUpload(item);
      item = { ...item, uploadId: created.uploadId };
      updateUpload(item.id, { uploadId: created.uploadId });
      if (cancelled.current.has(item.id)) {
        await removeServerUpload(created.uploadId);
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
            lastError = error;
            ensureUploadActive(item.id);
            await new Promise((resolve) => window.setTimeout(resolve, 800 * 2 ** attempt));
            ensureUploadActive(item.id);
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
      updateUpload(item.id, { state: "finalizing", message: text.finalizing });
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
          completionError = error;
          await new Promise((resolve) => window.setTimeout(resolve, 500 * 2 ** attempt));
        }
      }
      if (!result) throw completionError ?? new Error("Abschlussbestätigung fehlt");
      updateUpload(item.id, { state: "complete", progress: 100, message: text.savedAs(result.name) });
    } catch (error) {
      if (error instanceof HttpError && error.status === 401) handleSessionLost();
      const wasCancelled = cancelled.current.has(item.id);
      const wasPaused = paused.current.has(item.id);
      const scheduled = pendingUploads.current.includes(item.id);
      updateUpload(item.id, {
        state: wasCancelled ? "cancelled" : wasPaused ? "paused" : scheduled ? "queued" : "failed",
        message: wasPaused ? text.paused : scheduled ? text.waiting : error instanceof Error ? error.message : "Übertragung fehlgeschlagen",
      });
    }
  }

  async function drainUploadQueue() {
    if (processingQueue.current) return;
    processingQueue.current = true;
    try {
      while (pendingUploads.current.length) {
        if (!sessionRef.current) break;
        const id = pendingUploads.current.shift()!;
        const item = uploadRecords.current.get(id);
        if (!item || cancelled.current.has(id) || paused.current.has(id)) continue;
        await processUpload(item);
      }
    } finally {
      processingQueue.current = false;
    }
  }

  function scheduleUpload(id: string) {
    if (!pendingUploads.current.includes(id)) pendingUploads.current.push(id);
    void drainUploadQueue();
  }

  function queueFiles(files: File[]) {
    if (!files.length) return;
    const items = files.map((file) => ({ id: localQueueId(), file, state: "queued" as const, progress: 0, message: text.waiting }));
    for (const item of items) uploadRecords.current.set(item.id, item);
    setUploads((current) => [...current, ...items]);
    pendingUploads.current.push(...items.map((item) => item.id));
    void drainUploadQueue();
  }

  async function cancelUpload(item: UploadItem) {
    const current = uploadRecords.current.get(item.id) ?? item;
    if (current.state === "finalizing" || current.state === "complete") return;
    cancelled.current.add(item.id);
    paused.current.delete(item.id);
    pendingUploads.current = pendingUploads.current.filter((id) => id !== item.id);
    abortUploadWork(item.id);
    if (current.uploadId && sessionRef.current) {
      await removeServerUpload(current.uploadId);
    }
    updateUpload(item.id, { state: "cancelled", message: text.cancelled });
  }

  function pauseUpload(item: UploadItem) {
    const current = uploadRecords.current.get(item.id) ?? item;
    if (current.state !== "queued" && current.state !== "uploading") return;
    paused.current.add(item.id);
    abortUploadWork(item.id);
    updateUpload(item.id, { state: "paused", message: text.paused });
  }

  function resumeUpload(item: UploadItem) {
    if (cancelled.current.has(item.id)) return;
    paused.current.delete(item.id);
    updateUpload(item.id, { state: "queued", message: text.waiting });
    scheduleUpload(item.id);
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
          <button type="submit" className="primary-button">{text.connect}</button>
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
                <article className={`upload-item ${item.state}`} key={item.id}>
                  <header className="upload-heading">
                    <div><strong><bdi className="untrusted-name">{item.file.name}</bdi></strong><span>{formatBytes(item.file.size)} · {item.message}</span></div>
                    <b className="upload-state">{text.uploadState(item.state)}</b>
                  </header>
                  <div className="upload-progress"><progress aria-label={`${item.file.name}: ${Math.round(item.progress)} Prozent`} max={100} value={item.progress} /><span>{Math.round(item.progress)} %</span></div>
                  {(item.state === "queued" || item.state === "uploading") && <div className="upload-actions"><button onClick={() => pauseUpload(item)}>{text.pause}</button><button onClick={() => cancelUpload(item)}>{text.cancel}</button></div>}
                  {item.state === "paused" && <div className="upload-actions"><button onClick={() => resumeUpload(item)}>{text.resume}</button><button onClick={() => cancelUpload(item)}>{text.cancel}</button></div>}
                  {item.state === "failed" && <button className="retry-button" onClick={() => scheduleUpload(item.id)}>{text.retry}</button>}
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
