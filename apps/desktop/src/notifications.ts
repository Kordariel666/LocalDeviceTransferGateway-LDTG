import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { TransferChangedEvent, TransferState } from "@dmdc/shared";

export type BatchNotificationOutcome = "complete" | "failed";
export type BatchSettlement = BatchNotificationOutcome | "cancelled";
export type DesktopNotificationKind = BatchNotificationOutcome | "network-lost";

export type TransferNotificationState = {
  serviceId: string | null;
  transfers: Map<string, TransferState>;
};

export function createTransferNotificationState(): TransferNotificationState {
  return { serviceId: null, transfers: new Map() };
}

export function trackTransferNotification(
  current: TransferNotificationState,
  event: TransferChangedEvent,
): { state: TransferNotificationState; outcome: BatchSettlement | null } {
  const transfers = current.serviceId === event.serviceId
    ? new Map(current.transfers)
    : new Map<string, TransferState>();

  if (event.transfer.state === "active") {
    transfers.set(event.transfer.id, "active");
    return { state: { serviceId: event.serviceId, transfers }, outcome: null };
  }

  // A terminal event is only part of a locally observed batch if its active
  // event was seen first. Snapshot hydration therefore never creates a toast.
  if (!transfers.has(event.transfer.id)) {
    return { state: { serviceId: event.serviceId, transfers }, outcome: null };
  }

  transfers.set(event.transfer.id, event.transfer.state);
  if ([...transfers.values()].some((state) => state === "active")) {
    return { state: { serviceId: event.serviceId, transfers }, outcome: null };
  }

  const states = [...transfers.values()];
  const failed = states.some((state) => state === "failed" || state === "expired");
  const complete = states.every((state) => state === "complete");
  return {
    state: { serviceId: event.serviceId, transfers: new Map() },
    outcome: failed ? "failed" : complete ? "complete" : "cancelled",
  };
}

function notificationContent(kind: DesktopNotificationKind): { title: string; body: string } {
  if (kind === "complete") {
    return {
      title: "DMDC · Batch abgeschlossen",
      body: "Alle Übertragungen des aktuellen Batches wurden abgeschlossen.",
    };
  }
  if (kind === "failed") {
    return {
      title: "DMDC · Batch fehlgeschlagen",
      body: "Mindestens eine Übertragung des aktuellen Batches ist fehlgeschlagen.",
    };
  }
  return {
    title: "DMDC · Netzwerkverbindung verloren",
    body: "Der lokale Transferdienst wurde beendet, weil das verwendete Netzwerk nicht mehr verfügbar ist.",
  };
}

export function createDesktopNotifier() {
  let permissionGranted: boolean | undefined;

  return async (kind: DesktopNotificationKind): Promise<boolean> => {
    try {
      if (permissionGranted === undefined) {
        permissionGranted = await isPermissionGranted();
        if (!permissionGranted) permissionGranted = await requestPermission() === "granted";
      }
      if (!permissionGranted) return false;
      sendNotification(notificationContent(kind));
      return true;
    } catch {
      // Notifications are best effort and must never disturb local transfers.
      return false;
    }
  };
}

export const notifyDesktop = createDesktopNotifier();
