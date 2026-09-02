import { describe, expect, it } from "vitest";
import type { ServiceStatus } from "@dmdc/shared";
import { applySessionsChanged, applyTransferChanged } from "./statusEvents";

const running: ServiceStatus = {
  state: "running",
  serviceId: "service-1",
  url: "http://192.168.1.2:8765/",
  accessCode: "12345678",
  startedAt: "2026-09-02T10:00:00Z",
  activeTransfers: 0,
  sessions: [],
  transfers: [],
  error: null,
};

describe("status event reducers", () => {
  it("applies transfer progress only to the matching service instance", () => {
    const transfer = {
      id: "transfer-1",
      sessionId: "session-1",
      direction: "download" as const,
      name: "foto.jpg",
      startedAt: "2026-09-02T10:00:00Z",
      lastProgressAt: "2026-09-02T10:00:01Z",
      finishedAt: null,
      transferredBytes: 1024,
      totalBytes: 4096,
      bytesPerSecond: 1024,
      speedSampleCount: 1,
      state: "active" as const,
      updatedAt: "2026-09-02T10:00:01Z",
    };
    const updated = applyTransferChanged(running, { serviceId: "service-1", transfer });
    expect(updated.transfers).toEqual([transfer]);
    expect(updated.activeTransfers).toBe(1);
    expect(applyTransferChanged(updated, {
      serviceId: "stale-service",
      transfer: { ...transfer, transferredBytes: 4096, state: "complete" },
    })).toBe(updated);
  });

  it("applies session upserts, removals and resets without a full status response", () => {
    const session = {
      id: "session-1",
      address: "192.168.1.10",
      deviceName: null,
      clientName: "Safari auf iPhone",
      createdAt: "2026-09-02T10:00:00Z",
      lastActivity: "2026-09-02T10:00:01Z",
    };
    const added = applySessionsChanged(running, {
      kind: "upsert",
      serviceId: "service-1",
      session,
    });
    expect(added.sessions).toEqual([session]);
    const removed = applySessionsChanged(added, {
      kind: "remove",
      serviceId: "service-1",
      ids: [session.id],
    });
    expect(removed.sessions).toEqual([]);
    expect(applySessionsChanged(added, {
      kind: "reset",
      serviceId: "service-1",
    }).sessions).toEqual([]);
  });
});
