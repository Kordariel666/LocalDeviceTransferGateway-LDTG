import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { AppSnapshot, TransferDirection, TransferInfo, TransferState } from "@ldtg/shared";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  open: vi.fn(),
  ask: vi.fn(),
  save: vi.fn(),
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
  listeners: new Map<string, (event?: { payload: unknown }) => void>(),
}));

const snapshot: AppSnapshot = {
  appVersion: "0.2.0-rc.1",
  configurationWarning: null,
  settings: {
    version: 4,
    profiles: [{
      id: "00000000-0000-4000-8000-000000000001",
      name: "Standard",
      downloadShare: { enabled: false, path: "" },
      uploadShare: { enabled: false, path: "" },
      overrides: { network: null, port: null, limits: null },
    }],
    activeProfileId: "00000000-0000-4000-8000-000000000001",
    preferredAdapterId: null,
    port: 8765,
    maxUploadBytes: 20 * 1024 ** 3,
    maxInboxBytes: 100 * 1024 ** 3,
    maxInboxFiles: 10_000,
    idleTimeoutMinutes: null,
    trustedNetworks: [],
  },
  service: { state: "stopped", serviceId: null, url: null, accessCode: null, startedAt: null, activeTransfers: 0, sessions: [], transfers: [], error: null },
  networks: [{ id: "lan", name: "WLAN", profileName: "Heimnetz", address: "192.168.1.2", prefixLength: 24, networkId: "heim", category: "Öffentlich", profileResolved: true, preferred: true }],
  firewall: { configured: false, programPath: null, port: null, detail: "Regel fehlt" },
};

let currentSnapshot = snapshot;

function transfer(
  id: string,
  name: string,
  direction: TransferDirection,
  state: TransferState,
): TransferInfo {
  const active = state === "active";
  return {
    id,
    sessionId: "session-1",
    direction,
    name,
    startedAt: "2026-09-03T10:00:00Z",
    lastProgressAt: "2026-09-03T10:00:04Z",
    finishedAt: active ? null : "2026-09-03T10:00:05Z",
    transferredBytes: state === "complete" ? 10 : 5,
    totalBytes: 10,
    bytesPerSecond: active ? 5 : null,
    speedSampleCount: active ? 1 : 2,
    state,
    updatedAt: active ? "2026-09-03T10:00:04Z" : "2026-09-03T10:00:05Z",
  };
}

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: mocks.ask, open: mocks.open, save: mocks.save }));
vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: mocks.isPermissionGranted,
  requestPermission: mocks.requestPermission,
  sendNotification: mocks.sendNotification,
}));

import { App } from "./DesktopApp";

beforeEach(() => {
  currentSnapshot = structuredClone(snapshot);
  mocks.listeners.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(async (command: string, args?: { settings?: unknown; networkId?: string | null }) => {
    if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
    if (command === "get_service_status") return structuredClone(currentSnapshot.service);
    if (command === "clear_transfer_history") {
      currentSnapshot.service.transfers = currentSnapshot.service.transfers.filter((item) => item.state === "active");
      currentSnapshot.service.activeTransfers = currentSnapshot.service.transfers.length;
      return structuredClone(currentSnapshot.service);
    }
    if (command === "validate_share_settings") return { downloadError: null, uploadError: null, overlapError: null };
    if (command === "save_settings") {
      currentSnapshot.settings = structuredClone(args?.settings) as AppSnapshot["settings"];
      return structuredClone(currentSnapshot.settings);
    }
    if (command === "forget_trusted_network") {
      currentSnapshot.settings.trustedNetworks = args?.networkId === null
        ? []
        : currentSnapshot.settings.trustedNetworks.filter((network) => network.id !== args?.networkId);
      return structuredClone(currentSnapshot.settings);
    }
    return undefined;
  });
  mocks.listen.mockReset();
  mocks.listen.mockImplementation(async (event: string, callback: (event?: { payload: unknown }) => void) => {
    mocks.listeners.set(event, callback);
    return () => mocks.listeners.delete(event);
  });
  mocks.open.mockReset();
  mocks.ask.mockReset();
  mocks.save.mockReset();
  mocks.isPermissionGranted.mockReset();
  mocks.isPermissionGranted.mockResolvedValue(true);
  mocks.requestPermission.mockReset();
  mocks.sendNotification.mockReset();
});

afterEach(() => {
  cleanup();
  vi.clearAllTimers();
  vi.useRealTimers();
});

describe("Desktop-Dashboard", () => {
  it("zeigt getrennte Upload- und Downloadfreigaben und blockiert öffentliche Profile nicht", async () => {
    render(<App />);
    expect(await screen.findByText("Dateien fürs Handy")).toBeTruthy();
    expect(screen.getByText("Dateien vom Handy")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Dienst starten" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    expect(await screen.findByText(/Öffentlich/)).toBeTruthy();
  });

  it("dupliziert, benennt und löscht gespeicherte Freigabeprofile", async () => {
    mocks.ask.mockResolvedValue(true);
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Freigaben" }));

    fireEvent.click(screen.getByRole("button", { name: "Profil duplizieren" }));
    const selector = screen.getByRole("combobox", { name: "Aktives Profil" }) as HTMLSelectElement;
    expect(selector.options).toHaveLength(2);
    expect(selector.selectedOptions[0].textContent).toBe("Standard Kopie");

    fireEvent.change(screen.getByRole("textbox", { name: "Profilname" }), {
      target: { value: "Fotos" },
    });
    expect(selector.selectedOptions[0].textContent).toBe("Fotos");

    fireEvent.click(screen.getByRole("button", { name: "Freigaben speichern" }));
    await waitFor(() => expect(currentSnapshot.settings.profiles).toHaveLength(2));
    expect(currentSnapshot.settings.profiles[1].name).toBe("Fotos");

    fireEvent.click(screen.getByRole("button", { name: "Profil löschen" }));
    await waitFor(() => expect(selector.options).toHaveLength(1));
    expect(mocks.ask).toHaveBeenCalledWith(expect.stringContaining("Fotos"), expect.objectContaining({
      kind: "warning",
    }));
  });

  it("zeigt beschädigte Einstellungen als persistente Warnung", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.configurationWarning = "Die gespeicherten Einstellungen sind beschädigt; die vorhandene settings.json wurde unverändert behalten.";
    render(<App />);
    expect((await screen.findByRole("alert")).textContent).toContain("settings.json wurde unverändert behalten");
    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    expect((screen.getByRole("button", { name: "Einstellungen speichern" }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByText("Sichere Standardwerte können als neue Konfiguration gespeichert werden.")).toBeTruthy();
  });

  it("zeigt einen fehlgeschlagenen ersten Snapshot mit funktionierendem Retry", async () => {
    mocks.invoke.mockRejectedValueOnce("Snapshot vorübergehend nicht verfügbar");
    render(<App />);
    expect(await screen.findByText("Snapshot vorübergehend nicht verfügbar")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Erneut versuchen" }));
    expect(await screen.findByRole("button", { name: "Dienst starten" })).toBeTruthy();
  });

  it("kennzeichnet Änderungen und aktiviert Speichern nur solange der Entwurf abweicht", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Netzwerk & Sicherheit" }));
    const saveButton = screen.getByRole("button", { name: "Einstellungen speichern" }) as HTMLButtonElement;
    expect(saveButton.disabled).toBe(true);

    fireEvent.change(screen.getByRole("spinbutton", { name: /TCP-Port/ }), { target: { value: "9123" } });

    expect((await screen.findAllByText("Ungespeicherte Änderungen")).length).toBeGreaterThan(0);
    expect(saveButton.disabled).toBe(false);
    fireEvent.change(screen.getByRole("spinbutton", { name: /TCP-Port/ }), { target: { value: "8765" } });
    await waitFor(() => expect(saveButton.disabled).toBe(true));
    expect(screen.queryByText("Ungespeicherte Änderungen")).toBeNull();

    fireEvent.change(screen.getByRole("spinbutton", { name: /TCP-Port/ }), { target: { value: "9123" } });
    await waitFor(() => expect(saveButton.disabled).toBe(false));
    const unload = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(unload);
    expect(unload.defaultPrevented).toBe(true);

    fireEvent.click(saveButton);
    await waitFor(() => expect(saveButton.disabled).toBe(true));
    expect(screen.getByText("Alle Änderungen sind gespeichert.")).toBeTruthy();
  });

  it("zeigt Port- und Größenfehler direkt am verantwortlichen Feld", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.settings.maxInboxFiles = 0;
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Netzwerk & Sicherheit" }));

    const port = screen.getByRole("spinbutton", { name: /TCP-Port/ });
    fireEvent.change(port, { target: { value: "80" } });
    expect(port.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByText("Der Port muss eine ganze Zahl zwischen 1024 und 65535 sein.")).toBeTruthy();

    fireEvent.change(screen.getByRole("combobox", { name: /Uploadlimit pro Datei/ }), { target: { value: "100" } });
    fireEvent.change(screen.getByRole("combobox", { name: /Gesamtspeicher im Upload-Eingang/ }), { target: { value: "25" } });
    expect(screen.getByText("Das Limit pro Datei darf das Gesamtlimit des Upload-Eingangs nicht überschreiten.")).toBeTruthy();
    expect(screen.getByText("Das Gesamtlimit muss mindestens so groß wie das Limit pro Datei sein.")).toBeTruthy();
    expect(screen.getByText("Das Dateilimit muss eine positive ganze Zahl sein.")).toBeTruthy();
    expect(screen.getByRole("combobox", { name: /Dateien im Upload-Eingang/ }).getAttribute("aria-invalid")).toBe("true");
    expect((screen.getByRole("button", { name: "Einstellungen speichern" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("zeigt kanonische Freigabeüberschneidungen aus der Backendprüfung", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.settings.profiles[0].downloadShare = { enabled: true, path: "C:\\Daten" };
    currentSnapshot.settings.profiles[0].uploadShare = { enabled: true, path: "C:\\Daten\\Eingang" };
    mocks.invoke.mockImplementation(async (command: string) => {
      if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
      if (command === "get_service_status") return structuredClone(currentSnapshot.service);
      if (command === "validate_share_settings") return {
        downloadError: null,
        uploadError: null,
        overlapError: "Downloadfreigabe und Upload-Eingang müssen vollständig getrennte Ordner sein.",
      };
      return undefined;
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Freigaben" }));

    expect(await screen.findByText("Downloadfreigabe und Upload-Eingang müssen vollständig getrennte Ordner sein.")).toBeTruthy();
    expect(mocks.invoke).toHaveBeenCalledWith("validate_share_settings", { settings: currentSnapshot.settings });
  });

  it("zeigt die vom Backend gelieferte Buildversion statt des Konfigurationsschemas", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.appVersion = "9.8.7-test";
    currentSnapshot.settings.version = 42;
    render(<App />);

    const sidebar = await screen.findByRole("complementary");
    expect(within(sidebar).getByText("App-Version")).toBeTruthy();
    expect(within(sidebar).getByText("9.8.7-test")).toBeTruthy();

    fireEvent.click(await screen.findByRole("button", { name: "Diagnose" }));

    expect((await screen.findAllByText("9.8.7-test")).length).toBe(2);
    expect(screen.getAllByText("App-Version").length).toBe(2);
  });

  it("ordnet die laufende Dienst-URL nur einer exakt gleichen IP-Adresse zu", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.networks = [
      { ...currentSnapshot.networks[0], id: "short", name: "Falsches Netz", address: "192.168.1.2" },
      { ...currentSnapshot.networks[0], id: "exact", name: "Exaktes Netz", address: "192.168.1.20" },
    ];
    currentSnapshot.service = {
      ...currentSnapshot.service,
      state: "running",
      serviceId: "service",
      url: "http://192.168.1.20:8765/",
    };
    render(<App />);
    expect(await screen.findByText("Exaktes Netz")).toBeTruthy();
  });

  it("behält ungespeicherte Freigaben, Ordner und Einstellungen bei Hintergrundereignissen", async () => {
    vi.useFakeTimers();
    await act(async () => {
      render(<App />);
      await Promise.resolve();
      await Promise.resolve();
    });

    fireEvent.click(screen.getByRole("button", { name: "Freigaben" }));
    const downloadToggle = screen.getAllByRole("checkbox")[0] as HTMLInputElement;
    fireEvent.click(downloadToggle);
    expect(downloadToggle.checked).toBe(true);

    mocks.open.mockResolvedValueOnce("C:\\Handy-Freigabe");
    await act(async () => {
      fireEvent.click(screen.getAllByRole("button", { name: "Ordner auswählen" })[0]);
      await Promise.resolve();
    });
    expect(screen.getByText("C:\\Handy-Freigabe")).toBeTruthy();
    expect(screen.getAllByText("Ungespeicherte Änderungen").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    const port = screen.getByRole("spinbutton", { name: /TCP-Port/ }) as HTMLInputElement;
    fireEvent.change(port, { target: { value: "9123" } });
    expect(port.value).toBe("9123");

    await act(async () => {
      mocks.listeners.get("service-status-changed")?.();
      await vi.advanceTimersByTimeAsync(200);
    });

    fireEvent.click(screen.getByRole("button", { name: "Freigaben" }));
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText("C:\\Handy-Freigabe")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    expect((screen.getByRole("spinbutton", { name: /TCP-Port/ }) as HTMLInputElement).value).toBe("9123");
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "get_app_snapshot")).toHaveLength(1);
    expect(mocks.invoke.mock.calls.some(([command]) => command === "get_service_status")).toBe(true);
  });

  it("wendet Sitzungs- und Transferereignisse ohne vollständige Statusabfrage an", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
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
    render(<App />);
    await screen.findByText("Mit dem Handy verbinden");

    await act(async () => {
      mocks.listeners.get("sessions-changed")?.({
        payload: {
          kind: "upsert",
          serviceId: "service-1",
          session: {
            id: "session-1",
            address: "192.168.1.10",
            deviceName: "Mein Handy",
            clientName: "Chrome auf Android",
            createdAt: "2026-09-02T10:00:00Z",
            lastActivity: "2026-09-02T10:00:01Z",
          },
        },
      });
      mocks.listeners.get("transfer-updated")?.({
        payload: {
          serviceId: "service-1",
          transfer: {
            id: "transfer-1",
            sessionId: "session-1",
            direction: "upload",
            name: "direkt.txt",
            startedAt: "2026-09-02T10:00:00Z",
            lastProgressAt: "2026-09-02T10:00:01Z",
            finishedAt: null,
            transferredBytes: 1024,
            totalBytes: 4096,
            bytesPerSecond: 1024,
            speedSampleCount: 1,
            state: "active",
            updatedAt: "2026-09-02T10:00:01Z",
          },
        },
      });
    });

    expect((await screen.findByText("Mein Handy")).closest("bdi")).toBeTruthy();
    expect(screen.getByText("Chrome auf Android")).toBeTruthy();
    expect(screen.getByText("192.168.1.10")).toBeTruthy();
    expect(screen.getByText("Verbunden seit")).toBeTruthy();
    expect(screen.getByText("Letzte Aktivität")).toBeTruthy();
    expect(screen.getByText("1 aktiv")).toBeTruthy();
    const device = screen.getByRole("button", { name: "Gerät „Mein Handy“ trennen" }).closest("article")!;
    expect(screen.getByText("direkt.txt")).toBeTruthy();
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "get_service_status")).toHaveLength(0);

    await act(async () => {
      mocks.listeners.get("transfer-updated")?.({
        payload: {
          serviceId: "service-1",
          transfer: {
            id: "transfer-1",
            sessionId: "session-1",
            direction: "upload",
            name: "direkt.txt",
            startedAt: "2026-09-02T10:00:00Z",
            lastProgressAt: "2026-09-02T10:00:02Z",
            finishedAt: "2026-09-02T10:00:02Z",
            transferredBytes: 4096,
            totalBytes: 4096,
            bytesPerSecond: 1024,
            speedSampleCount: 2,
            state: "complete",
            updatedAt: "2026-09-02T10:00:02Z",
          },
        },
      });
    });
    await waitFor(() => expect(within(device).getByText("0 aktiv")).toBeTruthy());
  });

  it("behält eine sparsame Statusabfrage als 30-Sekunden-Fallback bei", async () => {
    vi.useFakeTimers();
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
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
    await act(async () => {
      render(<App />);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByText(/Sehr kurze Übertragungen können direkt im Verlauf landen/)).toBeTruthy();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(29_999);
    });
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "get_service_status")).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "get_service_status")).toHaveLength(1);
  });

  it("zeigt eine Firewallregel erst nach bestätigter Backendprüfung als eingerichtet", async () => {
    const configured = {
      configured: true,
      programPath: "C:\\Programme\\LDTG\\LDTG.exe",
      port: 8765,
      detail: "Firewallregel ist für LDTG und TCP-Port 8765 eingerichtet.",
    };
    mocks.invoke.mockImplementation(async (command: string, args?: { settings?: unknown }) => {
      if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
      if (command === "get_service_status") return structuredClone(currentSnapshot.service);
      if (command === "save_settings") return args?.settings;
      if (command === "configure_firewall") return configured;
      return undefined;
    });

    render(<App />);
    await screen.findByText("Dateien fürs Handy");
    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Firewall einrichten" }));
    });

    expect(await screen.findByText(configured.detail)).toBeTruthy();
    expect(screen.getByText("Die Windows-Firewallregel wurde eingerichtet und erfolgreich überprüft.")).toBeTruthy();
    expect(mocks.invoke).toHaveBeenCalledWith("configure_firewall");
  });

  it("bestätigt ein noch nicht vertrautes öffentliches Netzwerk und startet danach", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.firewall = {
      configured: true,
      programPath: "C:\\Programme\\LDTG\\LDTG.exe",
      port: 8765,
      detail: "Firewallregel eingerichtet.",
    };
    currentSnapshot.settings.profiles[0].downloadShare = { enabled: true, path: "C:\\Freigabe" };
    mocks.ask.mockResolvedValue(true);
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
      if (command === "get_service_status") return structuredClone(currentSnapshot.service);
      if (command === "validate_share_settings") return { downloadError: null, uploadError: null, overlapError: null };
      if (command === "save_settings") return args?.settings;
      if (command === "start_service" && !args?.networkApproval) {
        throw {
          code: "NETWORK_UNTRUSTED",
          message: "Dieses Netzwerk ist noch nicht als vertrauenswürdig bestätigt.",
          context: { kind: "networkApproval", token: "network-token", networkName: "WLAN" },
        };
      }
      if (command === "start_service" && !args?.broadShareApproval) {
        throw {
          code: "BROAD_SHARE",
          message: "Eine sehr breit gewählte Freigabe muss ausdrücklich bestätigt werden.",
          context: { kind: "broadShareApproval", token: "share-token", path: "C:\\Freigabe|Fotos" },
        };
      }
      return undefined;
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Dienst starten" }));

    await waitFor(() => {
      expect(mocks.ask).toHaveBeenCalledTimes(2);
      expect(mocks.invoke).toHaveBeenCalledWith("start_service", {
        networkApproval: "network-token",
        broadShareApproval: "share-token",
      });
      expect(mocks.ask).toHaveBeenCalledWith(expect.stringContaining("C:\\Freigabe|Fotos"), expect.any(Object));
    });
  });

  it("warnt über den nativen Quit-Pfad vor dem Verwerfen ungespeicherter Änderungen", async () => {
    mocks.ask.mockResolvedValue(true);
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
      if (command === "get_service_status") return structuredClone(currentSnapshot.service);
      if (command === "validate_share_settings") return { downloadError: null, uploadError: null, overlapError: null };
      if (command === "quit_app" && !args?.discardUnsaved) {
        throw { code: "UNSAVED_CHANGES", message: "Es gibt ungespeicherte Änderungen.", context: null };
      }
      return undefined;
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Netzwerk & Sicherheit" }));
    fireEvent.change(screen.getByRole("spinbutton", { name: /TCP-Port/ }), { target: { value: "9123" } });
    await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("set_unsaved_changes", { dirty: true }));

    await act(async () => {
      mocks.listeners.get("quit-requested")?.();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(mocks.ask).toHaveBeenCalledWith(expect.stringContaining("ungespeicherte Änderungen"), expect.objectContaining({
      okLabel: "Verwerfen und beenden",
    }));
    expect(mocks.invoke).toHaveBeenCalledWith("quit_app", { force: false, discardUnsaved: true });
  });

  it("bestätigt aktive Übertragungen anhand des strukturierten Fehlercodes", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
      ...currentSnapshot.service,
      state: "running",
      serviceId: "service",
      activeTransfers: 1,
    };
    mocks.ask.mockResolvedValue(true);
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
      if (command === "get_service_status") return structuredClone(currentSnapshot.service);
      if (command === "validate_share_settings") return { downloadError: null, uploadError: null, overlapError: null };
      if (command === "stop_service" && !args?.force) {
        throw {
          code: "ACTIVE_TRANSFERS",
          message: "Mindestens eine Übertragung ist noch aktiv.",
          context: { kind: "activeTransfers", count: 1 },
        };
      }
      return undefined;
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Dienst stoppen" }));

    await waitFor(() => {
      expect(mocks.ask).toHaveBeenCalledWith(expect.stringContaining("Eine Übertragung ist aktiv"), expect.objectContaining({
        okLabel: "Trotzdem stoppen",
      }));
      expect(mocks.invoke).toHaveBeenCalledWith("stop_service", { force: true });
    });
  });

  it("behält Navigation und einsehbare Einstellungen auch beim laufenden Dienst", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
      state: "running",
      serviceId: "service",
      url: "http://192.168.1.2:8765/",
      accessCode: "12345678",
      startedAt: "2026-08-29T12:00:00Z",
      activeTransfers: 0,
      sessions: [],
      transfers: [],
      error: null,
    };

    render(<App />);
    expect(await screen.findByRole("button", { name: "Dienst stoppen" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Freigaben" }));
    expect(screen.getByText("Während des laufenden Dienstes einsehbar")).toBeTruthy();
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).disabled).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    expect((screen.getByRole("spinbutton", { name: /TCP-Port/ }) as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByRole("button", { name: "Dienst stoppen" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Übersicht" })).toBeTruthy();
  });

  it("zeigt echten Fortschritt für aktive Übertragungen ohne erfundene Steueraktionen", async () => {
    const now = Date.now();
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
      state: "running",
      serviceId: "service",
      url: "http://192.168.1.2:8765/",
      accessCode: "12345678",
      startedAt: "2026-08-29T12:00:00Z",
      activeTransfers: 1,
      sessions: [],
      transfers: [{
        id: "transfer",
        sessionId: "session-1",
        direction: "upload",
        name: "Urlaub-2026.zip",
        startedAt: new Date(now - 5_000).toISOString(),
        lastProgressAt: new Date(now - 1_000).toISOString(),
        finishedAt: null,
        transferredBytes: 1024 ** 2,
        totalBytes: 4 * 1024 ** 2,
        bytesPerSecond: 1024 ** 2,
        speedSampleCount: 4,
        state: "active",
        updatedAt: new Date(now - 1_000).toISOString(),
      }],
      error: null,
    };

    render(<App />);
    const progress = await screen.findByRole("progressbar", { name: /Urlaub-2026.zip/ });
    expect(progress.getAttribute("aria-valuenow")).toBe("25");
    expect(screen.getByText("25 %")).toBeTruthy();
    expect(screen.getByText(/1 MiB von 4 MiB/)).toBeTruthy();
    expect(screen.getByText("Geschwindigkeit 1 MiB/s")).toBeTruthy();
    expect(screen.getByText("Noch etwa 3 s")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Pausieren|Abbrechen/ })).toBeNull();
  });

  it("zeigt Start, Ende, Dauer und Ergebnis, filtert den Verlauf und leert nur fertige Einträge", async () => {
    const active = transfer("active", "Aktiv.bin", "upload", "active");
    const complete = transfer("complete", "Fertig.bin", "upload", "complete");
    const failed = transfer("failed", "Fehler.bin", "download", "failed");
    const cancelled = transfer("cancelled", "Abbruch.bin", "upload", "cancelled");
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
      ...currentSnapshot.service,
      state: "running",
      serviceId: "service",
      activeTransfers: 1,
      transfers: [active, complete, failed, cancelled],
    };

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /^Übertragungen/ }));

    expect(screen.getAllByText("Dauer 5 s")).toHaveLength(3);
    expect(screen.getAllByText(/^Gestartet:/)).toHaveLength(4);
    expect(screen.getAllByText(/^Beendet:/)).toHaveLength(3);
    expect(screen.getAllByText("Abgeschlossen").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Fehlgeschlagen").length).toBeGreaterThan(0);

    fireEvent.change(screen.getByRole("combobox", { name: "Richtung filtern" }), {
      target: { value: "upload" },
    });
    expect(screen.getByText("Fertig.bin")).toBeTruthy();
    expect(screen.getByText("Abbruch.bin")).toBeTruthy();
    expect(screen.queryByText("Fehler.bin")).toBeNull();
    expect(screen.getByText("2 von 3 Einträgen")).toBeTruthy();

    fireEvent.change(screen.getByRole("combobox", { name: "Status filtern" }), {
      target: { value: "complete" },
    });
    expect(screen.getByText("Fertig.bin")).toBeTruthy();
    expect(screen.queryByText("Abbruch.bin")).toBeNull();
    expect(screen.getByText("1 von 3 Einträgen")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Verlauf leeren" }));
    expect(await screen.findByText("Noch kein Verlauf")).toBeTruthy();
    expect(screen.getByText("Aktiv.bin")).toBeTruthy();
    expect(mocks.invoke).toHaveBeenCalledWith("clear_transfer_history");
  });

  it("stoppt nur nach sichtbarer einmaliger Aktivierung und niemals allein durch ein Batchende", async () => {
    const activeTransfer = {
      id: "transfer",
      sessionId: "session-1",
      direction: "upload" as const,
      name: "Batch-Datei.bin",
      startedAt: "2026-09-03T10:00:00Z",
      lastProgressAt: "2026-09-03T10:00:01Z",
      finishedAt: null,
      transferredBytes: 5,
      totalBytes: 10,
      bytesPerSecond: 5,
      speedSampleCount: 1,
      state: "active" as const,
      updatedAt: "2026-09-03T10:00:01Z",
    };
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
      ...currentSnapshot.service,
      state: "running",
      serviceId: "service",
      activeTransfers: 1,
      transfers: [activeTransfer],
    };

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /^Übertragungen/ }));
    const stopAfterBatch = screen.getByRole("checkbox", {
      name: "Dienst stoppen, wenn alle aktuellen Übertragungen beendet sind",
    }) as HTMLInputElement;
    expect(stopAfterBatch.disabled).toBe(false);
    expect(stopAfterBatch.checked).toBe(false);

    await act(async () => {
      mocks.listeners.get("transfer-updated")?.({
        payload: {
          serviceId: "service",
          transfer: { ...activeTransfer, state: "complete", transferredBytes: 10 },
        },
      });
      await Promise.resolve();
    });
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "stop_service")).toHaveLength(0);

    await act(async () => {
      mocks.listeners.get("transfer-updated")?.({
        payload: { serviceId: "service", transfer: activeTransfer },
      });
    });
    fireEvent.click(stopAfterBatch);
    expect(stopAfterBatch.checked).toBe(true);

    await act(async () => {
      mocks.listeners.get("transfer-updated")?.({
        payload: {
          serviceId: "service",
          transfer: { ...activeTransfer, state: "complete", transferredBytes: 10 },
        },
      });
    });

    const nextTransfer = { ...activeTransfer, id: "transfer-2", name: "Zweite-Datei.bin" };
    await act(async () => {
      mocks.listeners.get("transfer-updated")?.({
        payload: { serviceId: "service", transfer: nextTransfer },
      });
    });
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 850));
    });
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "stop_service")).toHaveLength(0);

    await act(async () => {
      mocks.listeners.get("transfer-updated")?.({
        payload: {
          serviceId: "service",
          transfer: { ...nextTransfer, state: "complete", transferredBytes: 10 },
        },
      });
    });

    await waitFor(
      () => expect(mocks.invoke).toHaveBeenCalledWith("stop_service", { force: false }),
      { timeout: 2_000 },
    );
    expect(stopAfterBatch.checked).toBe(false);
    await waitFor(() => expect(mocks.sendNotification).toHaveBeenCalledWith(expect.objectContaining({
      title: expect.stringContaining("Batch abgeschlossen"),
    })), { timeout: 2_000 });
  });

  it("meldet einen echten Netzwerkverlust lokal und lässt den allgemeinen Idle-Timeout unverändert", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.service = {
      ...currentSnapshot.service,
      state: "running",
      serviceId: "service",
    };
    render(<App />);
    await screen.findByRole("button", { name: "Dienst stoppen" });

    await act(async () => {
      mocks.listeners.get("network-changed")?.({ payload: { available: false } });
      await Promise.resolve();
    });

    expect(mocks.sendNotification).toHaveBeenCalledWith(expect.objectContaining({
      title: expect.stringContaining("Netzwerkverbindung verloren"),
    }));
    expect(currentSnapshot.settings.idleTimeoutMinutes).toBeNull();
  });

  it("zeigt verfügbare und veraltete Vertrauensprofile und kann sie gezielt oder vollständig vergessen", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.settings.trustedNetworks = [
      {
        id: "heim",
        name: "Früheres Heimnetz",
        category: "Öffentlich",
        lastUsedAt: "2026-09-03T10:00:00Z",
      },
      {
        id: "nicht-mehr-vorhanden",
        name: "Altes WLAN",
        category: "Privat",
        lastUsedAt: null,
      },
    ];
    mocks.ask.mockResolvedValue(true);

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Netzwerk & Sicherheit" }));

    expect(await screen.findByText("Verfügbar")).toBeTruthy();
    expect(screen.getByText("Altes WLAN")).toBeTruthy();
    expect(screen.getByText("Nicht mehr auflösbar")).toBeTruthy();
    expect(screen.getByText("Noch nicht erfasst")).toBeTruthy();

    fireEvent.change(screen.getByRole("spinbutton", { name: /TCP-Port/ }), {
      target: { value: "9123" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Netzwerk „Altes WLAN“ vergessen" }));

    await waitFor(() => expect(screen.queryByText("Altes WLAN")).toBeNull());
    expect(mocks.invoke).toHaveBeenCalledWith("forget_trusted_network", {
      networkId: "nicht-mehr-vorhanden",
    });
    expect((screen.getByRole("spinbutton", { name: /TCP-Port/ }) as HTMLInputElement).value).toBe("9123");

    fireEvent.click(screen.getByRole("button", { name: "Alle Netzwerke vergessen" }));
    await waitFor(() => expect(screen.getByText("Noch kein Netzwerk bestätigt")).toBeTruthy());
    expect(mocks.ask).toHaveBeenCalledWith(expect.stringContaining("wirklich vergessen"), expect.objectContaining({
      kind: "warning",
    }));
    expect(mocks.invoke).toHaveBeenCalledWith("forget_trusted_network", { networkId: null });
  });

  it("markiert eine geänderte Netzwerkkategorie als erneut bestätigungspflichtig", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.settings.trustedNetworks = [{
      id: "heim",
      name: "Heimnetz",
      category: "Privat",
      lastUsedAt: "2026-09-03T10:00:00Z",
    }];

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Netzwerk & Sicherheit" }));

    expect(await screen.findByText("Kategorie geändert – neue Bestätigung nötig")).toBeTruthy();
    expect(screen.queryByText("Verfügbar")).toBeNull();
    expect(screen.getByText("Öffentlich")).toBeTruthy();
  });

  it("sperrt Änderungen an der Vertrauensliste während des laufenden Dienstes", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.settings.trustedNetworks = [{
      id: "heim",
      name: "Heimnetz",
      category: "Privat",
      lastUsedAt: "2026-09-03T10:00:00Z",
    }];
    currentSnapshot.service = {
      ...currentSnapshot.service,
      state: "running",
      serviceId: "service",
      url: "http://192.168.1.2:8765/",
      accessCode: "12345678",
      startedAt: "2026-09-03T10:00:00Z",
    };

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Netzwerk & Sicherheit" }));

    expect((screen.getByRole("button", { name: "Alle Netzwerke vergessen" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "Netzwerk „Heimnetz“ vergessen" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("Stoppen Sie den Dienst, bevor Sie die Vertrauensliste ändern.")).toBeTruthy();
  });
});
