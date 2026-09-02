export type UploadState =
  | "queued"
  | "uploading"
  | "paused"
  | "finalizing"
  | "complete"
  | "failed"
  | "cancelled";

export type UploadItem = {
  id: string;
  file: File;
  state: UploadState;
  progress: number;
  message: string;
  uploadId?: string;
  errorCode?: string;
};

export type UploadQueueState = {
  order: string[];
  items: Record<string, UploadItem>;
  pending: string[];
  sessionNotice: string | null;
};

export type UploadQueueSummary = {
  totalFiles: number;
  finishedFiles: number;
  totalBytes: number;
  transferredBytes: number;
  progress: number;
};

export type UploadQueueAction =
  | { type: "enqueue"; items: UploadItem[] }
  | { type: "claim"; id: string; message: string }
  | { type: "set-upload-id"; id: string; uploadId?: string; resetProgress?: boolean }
  | { type: "progress"; id: string; progress: number; message: string }
  | { type: "pause"; id: string; message: string }
  | { type: "pause-all"; message: string }
  | { type: "resume"; id: string; message: string }
  | { type: "resume-all"; message: string }
  | { type: "retry"; id: string; message: string }
  | { type: "retry-failed"; message: string }
  | { type: "cancel"; id: string; message: string }
  | { type: "remove"; id: string }
  | { type: "clear-finished" }
  | { type: "finalize"; id: string; message: string }
  | { type: "complete"; id: string; message: string }
  | { type: "fail"; id: string; message: string; errorCode?: string }
  | { type: "session-lost"; queuedMessage: string; pausedMessage: string; notice: string }
  | { type: "dismiss-session-notice" };

export const initialUploadQueue: UploadQueueState = {
  order: [],
  items: {},
  pending: [],
  sessionNotice: null,
};

function updateItem(
  state: UploadQueueState,
  id: string,
  update: (item: UploadItem) => UploadItem,
  pending = state.pending,
): UploadQueueState {
  const current = state.items[id];
  if (!current) return state;
  const next = update(current);
  if (next === current && pending === state.pending) return state;
  return { ...state, items: { ...state.items, [id]: next }, pending };
}

function without(pending: string[], id: string): string[] {
  return pending.includes(id) ? pending.filter((candidate) => candidate !== id) : pending;
}

function appendOnce(pending: string[], id: string): string[] {
  return pending.includes(id) ? pending : [...pending, id];
}

export function uploadQueueReducer(
  state: UploadQueueState,
  action: UploadQueueAction,
): UploadQueueState {
  switch (action.type) {
    case "enqueue": {
      const additions = action.items.filter((item) => !state.items[item.id]);
      if (!additions.length) return state;
      const items = { ...state.items };
      for (const item of additions) items[item.id] = item;
      return {
        ...state,
        order: [...state.order, ...additions.map((item) => item.id)],
        items,
        pending: [...state.pending, ...additions.map((item) => item.id)],
      };
    }
    case "claim":
      return updateItem(
        state,
        action.id,
        (item) => item.state === "queued"
          ? { ...item, state: "uploading", message: action.message, errorCode: undefined }
          : item,
        without(state.pending, action.id),
      );
    case "set-upload-id":
      return updateItem(state, action.id, (item) => ({
        ...item,
        uploadId: action.uploadId,
        progress: action.resetProgress ? 0 : item.progress,
      }));
    case "progress":
      return updateItem(state, action.id, (item) => item.state === "uploading"
        ? { ...item, progress: action.progress, message: action.message }
        : item);
    case "pause":
      return updateItem(
        state,
        action.id,
        (item) => item.state === "queued" || item.state === "uploading"
          ? { ...item, state: "paused", message: action.message }
          : item,
        without(state.pending, action.id),
      );
    case "pause-all": {
      const pausable = new Set(state.order.filter((id) => {
        const itemState = state.items[id]?.state;
        return itemState === "queued" || itemState === "uploading";
      }));
      if (!pausable.size) return state;
      const items = { ...state.items };
      for (const id of pausable) {
        items[id] = { ...items[id], state: "paused", message: action.message };
      }
      return {
        ...state,
        items,
        pending: state.pending.filter((id) => !pausable.has(id)),
      };
    }
    case "resume":
      if (state.items[action.id]?.state !== "paused") return state;
      return updateItem(
        state,
        action.id,
        (item) => item.state === "paused"
          ? { ...item, state: "queued", message: action.message, errorCode: undefined }
          : item,
        appendOnce(state.pending, action.id),
      );
    case "resume-all": {
      const resumable = state.order.filter((id) => state.items[id]?.state === "paused");
      if (!resumable.length) return state;
      const items = { ...state.items };
      let pending = state.pending;
      for (const id of resumable) {
        items[id] = { ...items[id], state: "queued", message: action.message, errorCode: undefined };
        pending = appendOnce(pending, id);
      }
      return { ...state, items, pending };
    }
    case "retry":
      if (state.items[action.id]?.state !== "failed") return state;
      return updateItem(
        state,
        action.id,
        (item) => item.state === "failed"
          ? { ...item, state: "queued", message: action.message, errorCode: undefined }
          : item,
        appendOnce(state.pending, action.id),
      );
    case "retry-failed": {
      const retryable = state.order.filter((id) => state.items[id]?.state === "failed");
      if (!retryable.length) return state;
      const items = { ...state.items };
      let pending = state.pending;
      for (const id of retryable) {
        items[id] = { ...items[id], state: "queued", message: action.message, errorCode: undefined };
        pending = appendOnce(pending, id);
      }
      return { ...state, items, pending };
    }
    case "cancel":
      return updateItem(
        state,
        action.id,
        (item) => item.state !== "finalizing" && item.state !== "complete" && item.state !== "cancelled"
          ? { ...item, state: "cancelled", message: action.message }
          : item,
        without(state.pending, action.id),
      );
    case "remove": {
      if (state.items[action.id]?.state !== "queued") return state;
      const items = { ...state.items };
      delete items[action.id];
      return {
        ...state,
        order: state.order.filter((id) => id !== action.id),
        items,
        pending: without(state.pending, action.id),
      };
    }
    case "clear-finished": {
      const finished = new Set(state.order.filter((id) => {
        const itemState = state.items[id]?.state;
        return itemState === "complete" || itemState === "cancelled";
      }));
      if (!finished.size) return state;
      const items = { ...state.items };
      for (const id of finished) delete items[id];
      return {
        ...state,
        order: state.order.filter((id) => !finished.has(id)),
        items,
        pending: state.pending.filter((id) => !finished.has(id)),
      };
    }
    case "finalize":
      return updateItem(state, action.id, (item) => item.state === "uploading"
        ? { ...item, state: "finalizing", message: action.message }
        : item);
    case "complete":
      return updateItem(state, action.id, (item) => item.state === "finalizing"
        ? { ...item, state: "complete", progress: 100, message: action.message, errorCode: undefined }
        : item);
    case "fail":
      return updateItem(state, action.id, (item) => item.state === "uploading" || item.state === "finalizing"
        ? { ...item, state: "failed", message: action.message, errorCode: action.errorCode }
        : item);
    case "session-lost": {
      const items = { ...state.items };
      const pending: string[] = [];
      let hasRecoverableUploads = false;
      for (const id of state.order) {
        const item = items[id];
        if (!item || item.state === "complete" || item.state === "cancelled") continue;
        hasRecoverableUploads = true;
        if (item.state === "paused") {
          items[id] = {
            ...item,
            uploadId: undefined,
            progress: 0,
            message: action.pausedMessage,
            errorCode: undefined,
          };
        } else {
          items[id] = {
            ...item,
            uploadId: undefined,
            state: "queued",
            progress: 0,
            message: action.queuedMessage,
            errorCode: undefined,
          };
          pending.push(id);
        }
      }
      return {
        ...state,
        items,
        pending,
        sessionNotice: hasRecoverableUploads ? action.notice : null,
      };
    }
    case "dismiss-session-notice":
      return state.sessionNotice === null ? state : { ...state, sessionNotice: null };
  }
}

export function uploadQueueItems(state: UploadQueueState): UploadItem[] {
  return state.order.flatMap((id) => state.items[id] ? [state.items[id]] : []);
}

export function nextQueuedUploadId(state: UploadQueueState): string | undefined {
  return state.pending.find((id) => state.items[id]?.state === "queued");
}

export function uploadQueueSummary(state: UploadQueueState): UploadQueueSummary {
  const items = uploadQueueItems(state);
  const totalBytes = items.reduce((sum, item) => sum + item.file.size, 0);
  const transferredBytesExact = items.reduce(
    (sum, item) => sum + item.file.size * Math.min(100, Math.max(0, item.progress)) / 100,
    0,
  );
  const progress = totalBytes > 0
    ? transferredBytesExact / totalBytes * 100
    : items.length > 0
      ? items.reduce((sum, item) => sum + Math.min(100, Math.max(0, item.progress)), 0) / items.length
      : 0;
  return {
    totalFiles: items.length,
    finishedFiles: items.filter((item) => item.state === "complete" || item.state === "cancelled").length,
    totalBytes,
    transferredBytes: Math.round(transferredBytesExact),
    progress,
  };
}

export function abortableDelay(milliseconds: number, signal: AbortSignal): Promise<void> {
  if (signal.aborted) return Promise.reject(signal.reason ?? new DOMException("Abgebrochen", "AbortError"));
  return new Promise((resolve, reject) => {
    const timeout = globalThis.setTimeout(done, milliseconds);
    signal.addEventListener("abort", aborted, { once: true });

    function cleanup() {
      globalThis.clearTimeout(timeout);
      signal.removeEventListener("abort", aborted);
    }

    function done() {
      cleanup();
      resolve();
    }

    function aborted() {
      cleanup();
      reject(signal.reason ?? new DOMException("Abgebrochen", "AbortError"));
    }
  });
}
