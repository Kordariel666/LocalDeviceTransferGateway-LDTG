import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TransferChangedEvent, TransferInfo, TransferState } from "@dmdc/shared";

const notificationMocks = vi.hoisted(() => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-notification", () => notificationMocks);

import {
  createDesktopNotifier,
  createTransferNotificationState,
  trackTransferNotification,
} from "./notifications";

function event(id: string, state: TransferState, serviceId = "service"): TransferChangedEvent {
  const transfer: TransferInfo = {
    id,
    sessionId: "session-1",
    direction: "upload",
    name: `${id}.bin`,
    startedAt: "2026-09-03T10:00:00Z",
    lastProgressAt: null,
    finishedAt: state === "active" ? null : "2026-09-03T10:00:01Z",
    transferredBytes: state === "complete" ? 10 : 0,
    totalBytes: 10,
    bytesPerSecond: null,
    speedSampleCount: 0,
    state,
    updatedAt: "2026-09-03T10:00:01Z",
  };
  return { serviceId, transfer };
}

beforeEach(() => {
  notificationMocks.isPermissionGranted.mockReset();
  notificationMocks.requestPermission.mockReset();
  notificationMocks.sendNotification.mockReset();
});

describe("Transfer-Benachrichtigungen", () => {
  it("meldet einen Batch erst, wenn alle beobachteten Übertragungen beendet sind", () => {
    let state = createTransferNotificationState();
    ({ state } = trackTransferNotification(state, event("a", "active")));
    ({ state } = trackTransferNotification(state, event("b", "active")));
    const first = trackTransferNotification(state, event("a", "complete"));
    expect(first.outcome).toBeNull();
    const last = trackTransferNotification(first.state, event("b", "complete"));
    expect(last.outcome).toBe("complete");
    expect(last.state.transfers.size).toBe(0);
  });

  it("wertet Fehler und Ablauf als fehlgeschlagen und ignoriert reine Snapshot-Endstände", () => {
    const ignored = trackTransferNotification(
      createTransferNotificationState(),
      event("old", "complete"),
    );
    expect(ignored.outcome).toBeNull();

    let state = ignored.state;
    ({ state } = trackTransferNotification(state, event("a", "active")));
    ({ state } = trackTransferNotification(state, event("b", "active")));
    ({ state } = trackTransferNotification(state, event("a", "complete")));
    const failed = trackTransferNotification(state, event("b", "expired"));
    expect(failed.outcome).toBe("failed");
  });

  it("vermischt Batches verschiedener Dienstläufe nicht", () => {
    let state = createTransferNotificationState();
    ({ state } = trackTransferNotification(state, event("old", "active", "first")));
    const staleTerminal = trackTransferNotification(state, event("new", "complete", "second"));
    expect(staleTerminal.outcome).toBeNull();
    expect(staleTerminal.state.serviceId).toBe("second");
    expect(staleTerminal.state.transfers.size).toBe(0);
  });

  it("fragt die Systemberechtigung einmalig an und sendet keine Dateinamen", async () => {
    notificationMocks.isPermissionGranted.mockResolvedValue(false);
    notificationMocks.requestPermission.mockResolvedValue("granted");
    const notify = createDesktopNotifier();

    await expect(notify("complete")).resolves.toBe(true);
    await expect(notify("failed")).resolves.toBe(true);
    await expect(notify("network-lost")).resolves.toBe(true);

    expect(notificationMocks.isPermissionGranted).toHaveBeenCalledTimes(1);
    expect(notificationMocks.requestPermission).toHaveBeenCalledTimes(1);
    expect(notificationMocks.sendNotification).toHaveBeenCalledTimes(3);
    expect(notificationMocks.sendNotification).toHaveBeenCalledWith(expect.objectContaining({
      title: expect.stringContaining("fehlgeschlagen"),
    }));
    expect(JSON.stringify(notificationMocks.sendNotification.mock.calls)).not.toContain(".bin");
  });
});
