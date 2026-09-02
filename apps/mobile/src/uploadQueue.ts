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
};

export type UploadQueueAction =
  | { type: "enqueue"; items: UploadItem[] }
  | { type: "claim"; id: string; message: string }
  | { type: "set-upload-id"; id: string; uploadId?: string; resetProgress?: boolean }
  | { type: "progress"; id: string; progress: number; message: string }
  | { type: "pause"; id: string; message: string }
  | { type: "resume"; id: string; message: string }
  | { type: "retry"; id: string; message: string }
  | { type: "cancel"; id: string; message: string }
  | { type: "finalize"; id: string; message: string }
  | { type: "complete"; id: string; message: string }
  | { type: "fail"; id: string; message: string; errorCode?: string }
  | { type: "session-lost"; queuedMessage: string; pausedMessage: string };

export const initialUploadQueue: UploadQueueState = {
  order: [],
  items: {},
  pending: [],
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
    case "cancel":
      return updateItem(
        state,
        action.id,
        (item) => item.state !== "finalizing" && item.state !== "complete" && item.state !== "cancelled"
          ? { ...item, state: "cancelled", message: action.message }
          : item,
        without(state.pending, action.id),
      );
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
      for (const id of state.order) {
        const item = items[id];
        if (!item || item.state === "complete" || item.state === "cancelled") continue;
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
      return { ...state, items, pending };
    }
  }
}

export function uploadQueueItems(state: UploadQueueState): UploadItem[] {
  return state.order.flatMap((id) => state.items[id] ? [state.items[id]] : []);
}

export function nextQueuedUploadId(state: UploadQueueState): string | undefined {
  return state.pending.find((id) => state.items[id]?.state === "queued");
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
