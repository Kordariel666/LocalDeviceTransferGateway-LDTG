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
    if (command === "validate_share_settings") return { downloadError: null, uploadError: null, overlapError: null };
    if (command === "save_settings") {
      currentSnapshot.settings = structuredClone(args?.settings) as AppSnapshot["settings"];
      return structuredClone(currentSnapshot.settings);
    }
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
    currentSnapshot.settings.downloadShare = { enabled: true, path: "C:\\Daten" };
    currentSnapshot.settings.uploadShare = { enabled: true, path: "C:\\Daten\\Eingang" };
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
    expect(screen.getAllByText("Ungespeicherte Änderungen").length).toBeGreaterThan(0);

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
    currentSnapshot.settings.downloadShare = { enabled: true, path: "C:\\Freigabe" };
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
