import { afterEach, describe, expect, it, vi } from "vitest";
import {
  abortableDelay,
  initialUploadQueue,
  nextQueuedUploadId,
  uploadQueueReducer,
  uploadQueueSummary,
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
    state = uploadQueueReducer(state, {
      type: "session-lost",
      queuedMessage: "wartet",
      pausedMessage: "pause",
      notice: "Sitzung verloren",
    });

    expect(state.items.a).toMatchObject({ state: "queued", uploadId: undefined, progress: 0 });
    expect(state.items.b).toMatchObject({ state: "paused", uploadId: undefined, progress: 0 });
    expect(state.pending).toEqual(["a", "c"]);
    expect(state.sessionNotice).toBe("Sitzung verloren");
  });

  it("meldet ohne wiederherstellbare Queue keinen scheinbaren Sitzungsverlust", () => {
    const state = uploadQueueReducer(initialUploadQueue, {
      type: "session-lost",
      queuedMessage: "wartet",
      pausedMessage: "pause",
      notice: "Sitzung verloren",
    });

    expect(state.sessionNotice).toBeNull();
  });

  it("pausiert und setzt den gesamten aktiven Batch in stabiler Reihenfolge fort", () => {
    let state = queued("a", "b", "c");
    state = uploadQueueReducer(state, { type: "claim", id: "a", message: "start" });
    state = uploadQueueReducer(state, { type: "pause", id: "c", message: "pause" });
    state = uploadQueueReducer(state, { type: "pause-all", message: "alle pausiert" });

    expect(state.order.map((id) => state.items[id].state)).toEqual(["paused", "paused", "paused"]);
    expect(state.pending).toEqual([]);

    state = uploadQueueReducer(state, { type: "resume-all", message: "wartet" });
    expect(state.order.map((id) => state.items[id].state)).toEqual(["queued", "queued", "queued"]);
    expect(state.pending).toEqual(["a", "b", "c"]);
  });

  it("stellt alle fehlgeschlagenen Dateien gemeinsam wieder an", () => {
    let state = queued("a", "b");
    for (const id of state.order) {
      state = uploadQueueReducer(state, { type: "claim", id, message: "start" });
      state = uploadQueueReducer(state, { type: "fail", id, message: "kaputt", errorCode: `ERR_${id}` });
    }
    state = uploadQueueReducer(state, { type: "retry-failed", message: "noch einmal" });

    expect(state.pending).toEqual(["a", "b"]);
    expect(state.items.a).toMatchObject({ state: "queued", errorCode: undefined });
    expect(state.items.b).toMatchObject({ state: "queued", errorCode: undefined });
  });

  it("entfernt nur wartende Dateien und räumt erledigte Einträge gesammelt auf", () => {
    let state = queued("active", "waiting", "done", "cancelled", "failed");
    state = uploadQueueReducer(state, { type: "claim", id: "active", message: "start" });
    expect(uploadQueueReducer(state, { type: "remove", id: "active" })).toBe(state);
    state = uploadQueueReducer(state, { type: "remove", id: "waiting" });
    state = uploadQueueReducer(state, { type: "claim", id: "done", message: "start" });
    state = uploadQueueReducer(state, { type: "finalize", id: "done", message: "final" });
    state = uploadQueueReducer(state, { type: "complete", id: "done", message: "fertig" });
    state = uploadQueueReducer(state, { type: "cancel", id: "cancelled", message: "weg" });
    state = uploadQueueReducer(state, { type: "claim", id: "failed", message: "start" });
    state = uploadQueueReducer(state, { type: "fail", id: "failed", message: "kaputt" });
    state = uploadQueueReducer(state, { type: "clear-finished" });

    expect(state.order).toEqual(["active", "failed"]);
    expect(state.items.waiting).toBeUndefined();
    expect(state.items.done).toBeUndefined();
    expect(state.items.cancelled).toBeUndefined();
    expect(state.items.failed.state).toBe("failed");
  });

  it("ignoriert Resume und Retry außerhalb ihrer erlaubten Zustände", () => {
    const state = queued("a");
    expect(uploadQueueReducer(state, { type: "resume", id: "a", message: "x" })).toBe(state);
    expect(uploadQueueReducer(state, { type: "retry", id: "a", message: "x" })).toBe(state);
  });
});

describe("uploadQueueSummary", () => {
  it("berechnet den Gesamtfortschritt nach Dateigröße gewichtet", () => {
    const state = uploadQueueReducer(initialUploadQueue, {
      type: "enqueue",
      items: [
        { ...upload("small", "complete"), file: new File(["12"], "small.bin"), progress: 100 },
        { ...upload("large", "uploading"), file: new File(["123456"], "large.bin"), progress: 50 },
      ],
    });

    expect(uploadQueueSummary(state)).toEqual({
      totalFiles: 2,
      finishedFiles: 1,
      totalBytes: 8,
      transferredBytes: 5,
      progress: 62.5,
    });
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
