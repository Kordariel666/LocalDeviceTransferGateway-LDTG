import type {
  ServiceStatus,
  SessionChangedEvent,
  TransferChangedEvent,
} from "@dmdc/shared";

export function applyTransferChanged(
  service: ServiceStatus,
  event: TransferChangedEvent,
): ServiceStatus {
  if (service.serviceId !== event.serviceId) return service;
  const transfers = service.transfers.slice();
  const index = transfers.findIndex((item) => item.id === event.transfer.id);
  if (index === -1) transfers.push(event.transfer);
  else transfers[index] = event.transfer;
  while (transfers.length > 100) {
    const inactive = transfers.findIndex((item) => item.state !== "active");
    if (inactive === -1) break;
    transfers.splice(inactive, 1);
  }
  return {
    ...service,
    transfers,
    activeTransfers: transfers.filter((item) => item.state === "active").length,
  };
}

export function applySessionsChanged(
  service: ServiceStatus,
  event: SessionChangedEvent,
): ServiceStatus {
  if (service.serviceId !== event.serviceId) return service;
  if (event.kind === "reset") return { ...service, sessions: [] };
  if (event.kind === "remove") {
    const removed = new Set(event.ids);
    return { ...service, sessions: service.sessions.filter((item) => !removed.has(item.id)) };
  }
  const sessions = service.sessions.slice();
  const index = sessions.findIndex((item) => item.id === event.session.id);
  if (index === -1) sessions.push(event.session);
  else sessions[index] = event.session;
  return { ...service, sessions };
}
