import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { AppSnapshot } from "@dmdc/shared";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  open: vi.fn(),
  ask: vi.fn(),
  save: vi.fn(),
  listeners: new Map<string, () => void>(),
}));

const snapshot: AppSnapshot = {
  appVersion: "0.1.3",
  configurationWarning: null,
  settings: {
    version: 2,
    downloadShare: { enabled: false, path: "" },
    uploadShare: { enabled: false, path: "" },
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

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: mocks.ask, open: mocks.open, save: mocks.save }));

import { App } from "./DesktopApp";

beforeEach(() => {
  currentSnapshot = structuredClone(snapshot);
  mocks.listeners.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(async (command: string, args?: { settings?: unknown }) => {
    if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
    if (command === "get_service_status") return structuredClone(currentSnapshot.service);
    if (command === "save_settings") return args?.settings;
    return undefined;
  });
  mocks.listen.mockReset();
  mocks.listen.mockImplementation(async (event: string, callback: () => void) => {
    mocks.listeners.set(event, callback);
    return () => mocks.listeners.delete(event);
  });
  mocks.open.mockReset();
  mocks.ask.mockReset();
  mocks.save.mockReset();
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

  it("zeigt beschädigte Einstellungen als persistente Warnung", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.configurationWarning = "Die gespeicherten Einstellungen sind beschädigt; die vorhandene settings.json wurde unverändert behalten.";
    render(<App />);
    expect((await screen.findByRole("alert")).textContent).toContain("settings.json wurde unverändert behalten");
  });

  it("zeigt einen fehlgeschlagenen ersten Snapshot mit funktionierendem Retry", async () => {
    mocks.invoke.mockRejectedValueOnce("Snapshot vorübergehend nicht verfügbar");
    render(<App />);
    expect(await screen.findByText("Snapshot vorübergehend nicht verfügbar")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Erneut versuchen" }));
    expect(await screen.findByRole("button", { name: "Dienst starten" })).toBeTruthy();
  });

  it("zeigt die vom Backend gelieferte Buildversion statt des Konfigurationsschemas", async () => {
    currentSnapshot = structuredClone(snapshot);
    currentSnapshot.appVersion = "9.8.7-test";
    currentSnapshot.settings.version = 42;
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "Diagnose" }));

    expect(await screen.findByText("9.8.7-test")).toBeTruthy();
    expect(screen.getByText("App-Version")).toBeTruthy();
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

    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    const port = screen.getByRole("spinbutton", { name: /TCP-Port/ }) as HTMLInputElement;
    fireEvent.change(port, { target: { value: "9123" } });
    expect(port.value).toBe("9123");

    await act(async () => {
      mocks.listeners.get("sessions-changed")?.();
      await vi.advanceTimersByTimeAsync(15_000);
    });

    fireEvent.click(screen.getByRole("button", { name: "Freigaben" }));
    expect((screen.getAllByRole("checkbox")[0] as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText("C:\\Handy-Freigabe")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Netzwerk & Sicherheit" }));
    expect((screen.getByRole("spinbutton", { name: /TCP-Port/ }) as HTMLInputElement).value).toBe("9123");
    expect(mocks.invoke.mock.calls.filter(([command]) => command === "get_app_snapshot")).toHaveLength(1);
    expect(mocks.invoke.mock.calls.some(([command]) => command === "get_service_status")).toBe(true);
  });

  it("zeigt eine Firewallregel erst nach bestätigter Backendprüfung als eingerichtet", async () => {
    const configured = {
      configured: true,
      programPath: "C:\\Programme\\DMDC\\DMDC.exe",
      port: 8765,
      detail: "Firewallregel ist für DMDC und TCP-Port 8765 eingerichtet.",
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
      programPath: "C:\\Programme\\DMDC\\DMDC.exe",
      port: 8765,
      detail: "Firewallregel eingerichtet.",
    };
    mocks.ask.mockResolvedValue(true);
    mocks.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "get_app_snapshot") return structuredClone(currentSnapshot);
      if (command === "get_service_status") return structuredClone(currentSnapshot.service);
      if (command === "save_settings") return args?.settings;
      if (command === "start_service" && !args?.networkApproval) {
        throw "NETWORK_UNTRUSTED|approval-token|WLAN";
      }
      return undefined;
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Dienst starten" }));

    await waitFor(() => {
      expect(mocks.ask).toHaveBeenCalledTimes(1);
      expect(mocks.invoke).toHaveBeenCalledWith("start_service", {
        networkApproval: "approval-token",
        broadShareApproval: null,
      });
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
        direction: "upload",
        name: "Urlaub-2026.zip",
        transferredBytes: 1024 ** 2,
        totalBytes: 4 * 1024 ** 2,
        state: "active",
        updatedAt: "2026-08-29T12:01:00Z",
      }],
      error: null,
    };

    render(<App />);
    const progress = await screen.findByRole("progressbar", { name: /Urlaub-2026.zip/ });
    expect(progress.getAttribute("aria-valuenow")).toBe("25");
    expect(screen.getByText("25 %")).toBeTruthy();
    expect(screen.getByText(/1 MiB von 4 MiB/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Pausieren|Abbrechen/ })).toBeNull();
  });
});
