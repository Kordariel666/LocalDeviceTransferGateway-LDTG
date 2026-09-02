import { afterEach, describe, expect, it, vi } from "vitest";
import {
  abortableDelay,
  initialUploadQueue,
  nextQueuedUploadId,
  uploadQueueReducer,
  type UploadItem,
  type UploadQueueState,
} from "./uploadQueue";

function upload(id: string, state: UploadItem["state"] = "queued"): UploadItem {
  return {
    id,
    file: new File([id], `${id}.bin`),
    state,
    progress: 0,
    message: state,
  };
}

function queued(...ids: string[]): UploadQueueState {
  return uploadQueueReducer(initialUploadQueue, {
    type: "enqueue",
    items: ids.map((id) => upload(id)),
  });
}

afterEach(() => vi.useRealTimers());

describe("uploadQueueReducer", () => {
  it("beansprucht Dateien stabil in Einfügereihenfolge", () => {
    let state = queued("a", "b");
    expect(nextQueuedUploadId(state)).toBe("a");
    state = uploadQueueReducer(state, { type: "claim", id: "a", message: "start" });
    expect(state.items.a.state).toBe("uploading");
    expect(nextQueuedUploadId(state)).toBe("b");
  });

  it.each(["pause", "cancel"] as const)("gibt nach %s sofort die nächste Datei frei", (type) => {
    let state = queued("a", "b");
    state = uploadQueueReducer(state, { type: "claim", id: "a", message: "start" });
    state = uploadQueueReducer(state, { type, id: "a", message: type });
    expect(state.items.a.state).toBe(type === "pause" ? "paused" : "cancelled");
    expect(nextQueuedUploadId(state)).toBe("b");
  });

  it("führt einen fehlgeschlagenen Upload sichtbar zurück über queued und uploading", () => {
    let state = queued("a");
    state = uploadQueueReducer(state, { type: "claim", id: "a", message: "start" });
    state = uploadQueueReducer(state, { type: "fail", id: "a", message: "kaputt", errorCode: "CHUNK_REJECTED" });
    expect(state.items.a).toMatchObject({ state: "failed", errorCode: "CHUNK_REJECTED" });
    state = uploadQueueReducer(state, { type: "retry", id: "a", message: "wartet" });
    expect(state.items.a).toMatchObject({ state: "queued", errorCode: undefined });
    state = uploadQueueReducer(state, { type: "claim", id: "a", message: "start" });
    expect(state.items.a.state).toBe("uploading");
  });

  it("bewahrt die Queue bei Sitzungsverlust und vergisst nur servergebundene IDs", () => {
    let state = queued("a", "b", "c");
    state = uploadQueueReducer(state, { type: "claim", id: "a", message: "start" });
    state = uploadQueueReducer(state, { type: "set-upload-id", id: "a", uploadId: "server-a" });
    state = uploadQueueReducer(state, { type: "pause", id: "b", message: "pause" });
    state = uploadQueueReducer(state, { type: "session-lost", queuedMessage: "wartet", pausedMessage: "pause" });

    expect(state.items.a).toMatchObject({ state: "queued", uploadId: undefined, progress: 0 });
    expect(state.items.b).toMatchObject({ state: "paused", uploadId: undefined, progress: 0 });
    expect(state.pending).toEqual(["a", "c"]);
  });

  it("ignoriert Resume und Retry außerhalb ihrer erlaubten Zustände", () => {
    const state = queued("a");
    expect(uploadQueueReducer(state, { type: "resume", id: "a", message: "x" })).toBe(state);
    expect(uploadQueueReducer(state, { type: "retry", id: "a", message: "x" })).toBe(state);
  });
});

describe("abortableDelay", () => {
  it.each([800, 1_600, 3_200])("bricht die Retry-Stufe %i ms ohne Rest-Timer ab", async (duration) => {
    vi.useFakeTimers();
    const controller = new AbortController();
    const waiting = abortableDelay(duration, controller.signal);
    controller.abort();
    await expect(waiting).rejects.toMatchObject({ name: "AbortError" });
    expect(vi.getTimerCount()).toBe(0);
  });
});
