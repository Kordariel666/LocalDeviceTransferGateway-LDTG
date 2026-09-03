import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  localStorage.clear();
});

function json(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
}

describe("mobile Oberfläche", () => {
  it("meldet sich mit dem separaten Code an und zeigt beide Rollen", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({ code: "AUTH_REQUIRED", message: "Bitte anmelden" }, 401))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(json({ serviceId: "dienst", csrfToken: "csrf", downloadEnabled: true, uploadEnabled: true, maxUploadBytes: 1024 }))
      .mockResolvedValueOnce(json({ path: "", entries: [], nextCursor: null }));
    render(<App />);
    const input = await screen.findByLabelText("Achtstelliger Zugangscode");
    await userEvent.type(screen.getByLabelText("Gerätename (optional)"), "Marias iPhone");
    await userEvent.type(input, "12345678");
    fireEvent.submit(input.closest("form")!);
    expect(await screen.findByText("Vom PC herunterladen")).toBeTruthy();
    expect(screen.getByText("Zum PC hochladen")).toBeTruthy();
    expect(fetchMock.mock.calls[1]?.[0]).toBe("/api/v1/auth");
    expect(JSON.parse(String(fetchMock.mock.calls[1]?.[1]?.body))).toEqual({
      code: "12345678",
      deviceName: "Marias iPhone",
    });
  });

  it("öffnet bei doppeltem Absenden höchstens eine Anmeldesitzung", async () => {
    let finishAuth!: (response: Response) => void;
    const authResponse = new Promise<Response>((resolve) => { finishAuth = resolve; });
    let sessionCalls = 0;
    let authCalls = 0;
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") {
        sessionCalls += 1;
        if (sessionCalls === 1) return json({ code: "AUTH_REQUIRED", message: "Bitte anmelden" }, 401);
        return json({ serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null });
      }
      if (input === "/api/v1/auth" && init?.method === "POST") {
        authCalls += 1;
        return authResponse;
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const input = await screen.findByLabelText("Achtstelliger Zugangscode");
    await userEvent.type(input, "12345678");
    const form = input.closest("form")!;
    fireEvent.submit(form);
    fireEvent.submit(form);
    expect(authCalls).toBe(1);
    expect((screen.getByRole("button", { name: "Verbinden" }) as HTMLButtonElement).disabled).toBe(true);
    finishAuth(new Response(null, { status: 204 }));
    expect(await screen.findByText("Dateien zum PC")).toBeTruthy();
  });

  it("trennt die lokale Sitzung auch bei nicht erreichbarem Dienst", async () => {
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/logout" && init?.method === "POST") throw new TypeError("Dienst nicht erreichbar");
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Trennen" }));
    expect(await screen.findByLabelText("Achtstelliger Zugangscode")).toBeTruthy();
    expect(screen.getByText(/Lokal getrennt.*Dienst nicht erreichbar/)).toBeTruthy();
  });

  it("zeigt bei reinem Upload-Eingang keine PC-Dateiliste und keine Verwaltungsaktionen", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce(json({
      serviceId: "dienst",
      csrfToken: "csrf",
      downloadEnabled: false,
      uploadEnabled: true,
      maxUploadBytes: null,
    }));
    render(<App />);
    expect(await screen.findByText("Dateien zum PC")).toBeTruthy();
    expect(screen.queryByText("Vom PC herunterladen")).toBeNull();
    expect(screen.queryByText(/löschen/i)).toBeNull();
    expect(screen.queryByText(/umbenennen/i)).toBeNull();
    await waitFor(() => expect(screen.getByText(/Vorhandene PC-Dateien bleiben verborgen/)).toBeTruthy());
  });

  it("zeigt eine laufende Uploaddatei mit Fortschritt und funktionsfähiger Pause", async () => {
    class PendingXhr {
      upload: { onprogress: ((event: ProgressEvent) => void) | null } = { onprogress: null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {}
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", PendingXhr);
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({
        serviceId: "dienst",
        csrfToken: "csrf",
        downloadEnabled: false,
        uploadEnabled: true,
        maxUploadBytes: 20 * 1024 ** 3,
      }))
      .mockResolvedValueOnce(json({
        uploadId: "upload",
        offset: 0,
        totalBytes: 4,
        chunkSize: 4,
        serviceId: "dienst",
        lastModified: 1,
      }))
      .mockResolvedValueOnce(json({
        uploadId: "upload",
        offset: 0,
        totalBytes: 4,
        chunkSize: 4,
        serviceId: "dienst",
        lastModified: 1,
      }));

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    const file = new File(["test"], "urlaub.zip", { type: "application/zip", lastModified: 1 });
    await userEvent.upload(picker, file);

    expect(await screen.findByText("urlaub.zip")).toBeTruthy();
    expect(screen.getByRole("progressbar", { name: /urlaub.zip/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Pausieren" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Pausieren" }));
    expect(await screen.findByText("Pausiert", { selector: ".upload-state" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Fortsetzen" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(fetchMock.mock.calls[2]?.[0]).toBe("/api/v1/uploads/upload");
  });

  it("verwirft eine ältere Ordnerantwort nach neuerer Navigation", async () => {
    let resolveA!: (response: Response) => void;
    let resolveB!: (response: Response) => void;
    const responseA = new Promise<Response>((resolve) => { resolveA = resolve; });
    const responseB = new Promise<Response>((resolve) => { resolveB = resolve; });
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input) => {
      const url = String(input);
      if (url === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: true, uploadEnabled: false, maxUploadBytes: null,
      });
      if (url.includes("path=A")) return responseA;
      if (url.includes("path=B")) return responseB;
      return json({
        path: "",
        entries: [
          { name: "A", path: "A", kind: "directory", size: 0, modifiedAt: null },
          { name: "B", path: "B", kind: "directory", size: 0, modifiedAt: null },
        ],
        nextCursor: null,
      });
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /A.*Öffnen/ }));
    fireEvent.click(screen.getByRole("button", { name: /B.*Öffnen/ }));
    resolveB(json({
      path: "B", entries: [{ name: "neu.txt", path: "B/neu.txt", kind: "file", size: 1, modifiedAt: null }], nextCursor: null,
    }));
    expect(await screen.findByText("neu.txt")).toBeTruthy();
    resolveA(json({
      path: "A", entries: [{ name: "alt.txt", path: "A/alt.txt", kind: "file", size: 1, modifiedAt: null }], nextCursor: null,
    }));
    await Promise.resolve();
    expect(screen.queryByText("alt.txt")).toBeNull();
    expect(screen.getByText("neu.txt")).toBeTruthy();
  });

  it("verwendet beim Nachladen den angewendeten statt des bearbeiteten Suchfilters", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: true, uploadEnabled: false, maxUploadBytes: null,
      }))
      .mockResolvedValueOnce(json({ path: "", query: "", entries: [], nextCursor: null, nextPage: null }))
      .mockResolvedValueOnce(json({
        path: "", query: "alt", entries: [{ name: "alt-1.txt", path: "alt-1.txt", kind: "file", size: 1, modifiedAt: null }], nextCursor: "cursor", nextPage: 1,
      }))
      .mockResolvedValueOnce(json({
        path: "", query: "alt", entries: [{ name: "alt-2.txt", path: "alt-2.txt", kind: "file", size: 1, modifiedAt: null }], nextCursor: null, nextPage: null,
      }));

    render(<App />);
    const search = await screen.findByLabelText("In diesem Ordner suchen");
    await userEvent.type(search, "alt");
    fireEvent.submit(search.closest("form")!);
    expect(await screen.findByText("alt-1.txt")).toBeTruthy();
    await userEvent.clear(search);
    await userEvent.type(search, "neu");
    fireEvent.click(screen.getByRole("button", { name: "Weitere Dateien anzeigen" }));
    expect(await screen.findByText("alt-2.txt")).toBeTruthy();

    const url = String(fetchMock.mock.calls[3]?.[0]);
    expect(url).toContain("cursor=cursor");
    expect(url).toContain("page=1");
    expect(url).toContain("q=alt");
    expect(url).not.toContain("q=neu");
  });

  it("übernimmt bei gleichen Metadaten niemals eine Upload-ID einer anderen Dateiauswahl", async () => {
    class PendingXhr {
      upload: { onprogress: ((event: ProgressEvent) => void) | null } = { onprogress: null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {}
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", PendingXhr);
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      }))
      .mockResolvedValueOnce(json({ uploadId: "upload-a", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }))
      .mockResolvedValueOnce(json({ uploadId: "upload-b", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }));
    localStorage.setItem("ldtg:dienst:gleich.bin:4:1", "stale-upload");

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "gleich.bin", { lastModified: 1 }));
    fireEvent.click(await screen.findByRole("button", { name: "Pausieren" }));
    expect(await screen.findByText("Pausiert", { selector: ".upload-state" })).toBeTruthy();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));

    await userEvent.upload(picker, new File(["BBBB"], "gleich.bin", { lastModified: 1 }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    const createCalls = fetchMock.mock.calls.filter(([url, init]) =>
      url === "/api/v1/uploads" && init?.method === "POST");
    expect(createCalls).toHaveLength(2);
    expect(fetchMock.mock.calls.some(([url]) => String(url).includes("stale-upload"))).toBe(false);
  });

  it("behält die serverseitige Upload-ID beim Pausieren während der Erstellung", async () => {
    class PendingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {}
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", PendingXhr);
    let finishCreate!: (response: Response) => void;
    const create = new Promise<Response>((resolve) => { finishCreate = resolve; });
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") return create;
      if (input === "/api/v1/uploads/upload-created") {
        return json({ uploadId: "upload-created", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "pause-create.bin", { lastModified: 1 }));
    await waitFor(() => expect(fetchMock.mock.calls.some(([url]) => url === "/api/v1/uploads")).toBe(true));
    fireEvent.click(await screen.findByRole("button", { name: "Pausieren" }));
    finishCreate(json({ uploadId: "upload-created", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }));

    expect(await screen.findByText("Pausiert", { selector: ".upload-state" })).toBeTruthy();
    await new Promise((resolve) => setTimeout(resolve, 0));
    fireEvent.click(screen.getByRole("button", { name: "Fortsetzen" }));
    await waitFor(() => expect(fetchMock.mock.calls.some(([url]) => url === "/api/v1/uploads/upload-created")).toBe(true));
    const creates = fetchMock.mock.calls.filter(([url, init]) => url === "/api/v1/uploads" && init?.method === "POST");
    expect(creates).toHaveLength(1);
  });

  it("dupliziert den Upload nicht bei sofortigem Fortsetzen einer laufenden Erstellung", async () => {
    class CompletingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 200;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader(name: string) { return name === "Upload-Offset" ? "4" : null; }
      send() { queueMicrotask(() => this.onload?.()); }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", CompletingXhr);
    let finishCreate!: (response: Response) => void;
    const create = new Promise<Response>((resolve) => { finishCreate = resolve; });
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") return create;
      if (input === "/api/v1/uploads/upload-resumed/complete" && init?.method === "POST") {
        return json({ name: "resume-create.bin" });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "resume-create.bin", { lastModified: 1 }));
    fireEvent.click(await screen.findByRole("button", { name: "Pausieren" }));
    fireEvent.click(await screen.findByRole("button", { name: "Fortsetzen" }));
    finishCreate(json({ uploadId: "upload-resumed", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }));

    expect(await screen.findByText("Abgeschlossen", { selector: ".upload-state" })).toBeTruthy();
    const creates = fetchMock.mock.calls.filter(([url, init]) => url === "/api/v1/uploads" && init?.method === "POST");
    expect(creates).toHaveLength(1);
    expect(fetchMock.mock.calls.some(([url, init]) =>
      url === "/api/v1/uploads/upload-resumed" && !init?.method)).toBe(false);
  });

  it("entfernt eine während der Erstellung abgebrochene Upload-ID nachträglich", async () => {
    let chunkSends = 0;
    class PendingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() { chunkSends += 1; }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", PendingXhr);
    let finishCreate!: (response: Response) => void;
    const create = new Promise<Response>((resolve) => { finishCreate = resolve; });
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") return create;
      if (input === "/api/v1/uploads/upload-cancelled" && init?.method === "DELETE") {
        return new Response(null, { status: 204 });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "cancel-create.bin", { lastModified: 1 }));
    await waitFor(() => expect(fetchMock.mock.calls.some(([url]) => url === "/api/v1/uploads")).toBe(true));
    fireEvent.click(await screen.findByRole("button", { name: "Abbrechen" }));
    finishCreate(json({ uploadId: "upload-cancelled", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }));

    expect(await screen.findByText("Abgebrochen", { selector: ".upload-state" })).toBeTruthy();
    await waitFor(() => expect(fetchMock.mock.calls.some(([url, init]) =>
      url === "/api/v1/uploads/upload-cancelled" && init?.method === "DELETE")).toBe(true));
    expect(chunkSends).toBe(0);
  });

  it("startet nach einer Pause im Retry-Backoff keinen weiteren Block", async () => {
    let sends = 0;
    class FailingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() { sends += 1; queueMicrotask(() => this.onerror?.()); }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", FailingXhr);
    vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      }))
      .mockResolvedValueOnce(json({ uploadId: "upload", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }));

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "backoff.bin", { lastModified: 1 }));
    await waitFor(() => expect(sends).toBe(1));
    await Promise.resolve();
    fireEvent.click(screen.getByRole("button", { name: "Pausieren" }));
    expect(await screen.findByText("Pausiert", { selector: ".upload-state" })).toBeTruthy();
    await new Promise((resolve) => setTimeout(resolve, 900));
    expect(sends).toBe(1);
  });

  it.each(["Pausieren", "Abbrechen"])("startet die nächste Datei nach %s im Retry-Backoff sofort", async (action) => {
    let sends = 0;
    class FirstFailureXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {
        sends += 1;
        if (sends === 1) queueMicrotask(() => this.onerror?.());
      }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", FirstFailureXhr);
    const createdNames: string[] = [];
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") {
        const name = (JSON.parse(String(init.body)) as { name: string }).name;
        createdNames.push(name);
        return json({ uploadId: `upload-${createdNames.length}`, offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      if (String(input).startsWith("/api/v1/uploads/") && init?.method === "DELETE") {
        return new Response(null, { status: 204 });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, [
      new File(["AAAA"], "erste.bin", { lastModified: 1 }),
      new File(["BBBB"], "zweite.bin", { lastModified: 2 }),
    ]);
    await waitFor(() => expect(sends).toBe(1));
    const first = screen.getByText("erste.bin").closest("article")!;
    const button = [...first.querySelectorAll("button")].find((candidate) => candidate.textContent === action)!;
    fireEvent.click(button);
    await waitFor(() => expect(createdNames).toEqual(["erste.bin", "zweite.bin"]), { timeout: 250 });
  });

  it("zeigt strukturierte PATCH-Fehler mit stabilem Code und startet Retry neu", async () => {
    let sends = 0;
    class StructuredErrorXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 422;
      responseText = JSON.stringify({ code: "CHUNK_REJECTED", message: "Block passt nicht zur Datei" });
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {
        sends += 1;
        if (sends === 1) queueMicrotask(() => this.onload?.());
      }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", StructuredErrorXhr);
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") {
        return json({ uploadId: "upload", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      if (input === "/api/v1/uploads/upload") {
        return json({ uploadId: "upload", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "fehler.bin", { lastModified: 1 }));
    const failed = (await screen.findByText("Fehlgeschlagen", { selector: ".upload-state" })).closest("article")!;
    expect(failed.dataset.errorCode).toBe("CHUNK_REJECTED");
    expect(failed.textContent).toContain("Block passt nicht zur Datei");
    fireEvent.click(screen.getByRole("button", { name: "Erneut versuchen" }));
    await waitFor(() => expect(failed.querySelector(".upload-state")?.textContent).toBe("Läuft"));
    expect(sends).toBe(2);
  });

  it("nimmt weitere Dateien während eines laufenden Uploads sichtbar in die Warteschlange auf", async () => {
    class PendingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {}
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", PendingXhr);
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      }))
      .mockResolvedValueOnce(json({ uploadId: "upload-a", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }));

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "erste.bin", { lastModified: 1 }));
    expect(await screen.findByText("erste.bin")).toBeTruthy();
    await userEvent.upload(picker, new File(["BBBB"], "zweite.bin", { lastModified: 2 }));

    const second = (await screen.findByText("zweite.bin")).closest("article")!;
    expect(second.querySelector(".upload-state")?.textContent).toBe("Wartet");
    const secondPause = [...second.querySelectorAll("button")].find((button) => button.textContent === "Pausieren")!;
    fireEvent.click(secondPause);
    expect(second.querySelector(".upload-state")?.textContent).toBe("Pausiert");
    const first = screen.getByText("erste.bin").closest("article")!;
    const firstPause = [...first.querySelectorAll("button")].find((button) => button.textContent === "Pausieren")!;
    fireEvent.click(firstPause);
    await waitFor(() => expect(second.querySelector(".upload-state")?.textContent).toBe("Pausiert"));
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("pausiert, setzt fort und kürzt einen laufenden Batch über die Sammelsteuerung", async () => {
    class PendingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {}
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", PendingXhr);
    vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") {
        return json({ uploadId: "upload-a", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      if (input === "/api/v1/uploads/upload-a") {
        return json({ uploadId: "upload-a", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, [
      new File(["AAAA"], "erste.bin", { lastModified: 1 }),
      new File(["BBBB"], "zweite.bin", { lastModified: 2 }),
    ]);
    await screen.findByText("Läuft", { selector: ".upload-state" });
    fireEvent.click(screen.getByRole("button", { name: "Alle pausieren" }));
    await waitFor(() => expect(screen.getAllByText("Pausiert", { selector: ".upload-state" })).toHaveLength(2));

    fireEvent.click(screen.getByRole("button", { name: "Alle fortsetzen" }));
    await screen.findByText("Läuft", { selector: ".upload-state" });
    const second = screen.getByText("zweite.bin").closest("article")!;
    expect(second.querySelector(".upload-state")?.textContent).toBe("Wartet");
    fireEvent.click(within(second).getByRole("button", { name: "Entfernen" }));
    expect(screen.queryByText("zweite.bin")).toBeNull();
    expect(screen.getByText("0 von 1 Dateien erledigt · 0 B von 4 B")).toBeTruthy();
  });

  it("macht eine bereits finalisierende Datei nicht mehr pausier- oder abbrechbar", async () => {
    class CompletingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 200;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader(name: string) { return name === "Upload-Offset" ? "4" : null; }
      send() { queueMicrotask(() => this.onload?.()); }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", CompletingXhr);
    let finish!: (response: Response) => void;
    const completion = new Promise<Response>((resolve) => { finish = resolve; });
    vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      }))
      .mockResolvedValueOnce(json({ uploadId: "upload", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 }))
      .mockReturnValueOnce(completion);

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "final.bin", { lastModified: 1 }));
    const finalizing = await screen.findByText("Wird abgeschlossen", { selector: ".upload-state" });
    const item = finalizing.closest("article")!;
    expect(item.querySelector("button")).toBeNull();
    finish(json({ name: "final.bin" }));
    expect(await screen.findByText("Abgeschlossen", { selector: ".upload-state" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Erledigte entfernen" }));
    expect(screen.queryByText("final.bin")).toBeNull();
  });

  it("wiederholt einen verlorenen Uploadabschluss ohne eine zweite Datei anzulegen", async () => {
    class CompletingXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 200;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader(name: string) { return name === "Upload-Offset" ? "4" : null; }
      send() { queueMicrotask(() => this.onload?.()); }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", CompletingXhr);
    let completionCalls = 0;
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") return json({
        serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null,
      });
      if (input === "/api/v1/uploads" && init?.method === "POST") {
        return json({ uploadId: "upload", offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      if (input === "/api/v1/uploads/upload/complete" && init?.method === "POST") {
        completionCalls += 1;
        if (completionCalls === 1) throw new TypeError("Antwort verloren");
        return json({ name: "verlustsicher.bin" });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "verlustsicher.bin", { lastModified: 1 }));
    expect(await screen.findByText("Wird abgeschlossen", { selector: ".upload-state" })).toBeTruthy();
    await new Promise((resolve) => setTimeout(resolve, 600));
    expect(await screen.findByText("Abgeschlossen", { selector: ".upload-state" })).toBeTruthy();
    expect(completionCalls).toBe(2);
    const creates = fetchMock.mock.calls.filter(([url, init]) => url === "/api/v1/uploads" && init?.method === "POST");
    expect(creates).toHaveLength(1);
    expect(String(creates[0][1]?.body)).toMatch(/"clientToken":"[0-9a-f]{32}"/);
  });

  it("behält die gesamte Uploadwarteschlange nach Sitzungsverlust und Anmeldung", async () => {
    let failFirst!: () => void;
    let sends = 0;
    class SessionLossXhr {
      upload = { onprogress: null as ((event: ProgressEvent) => void) | null };
      status = 0;
      responseText = "";
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      onabort: (() => void) | null = null;
      open() {}
      setRequestHeader() {}
      getResponseHeader() { return null; }
      send() {
        sends += 1;
        if (sends === 1) {
          failFirst = () => {
            this.status = 401;
            this.onload?.();
          };
        }
      }
      abort() { this.onabort?.(); }
    }
    vi.stubGlobal("XMLHttpRequest", SessionLossXhr);
    let sessionCalls = 0;
    let createCalls = 0;
    const fetchMock = vi.spyOn(globalThis, "fetch").mockImplementation(async (input, init) => {
      if (input === "/api/v1/session") {
        sessionCalls += 1;
        return json({
          serviceId: "dienst",
          csrfToken: `csrf-${sessionCalls}`,
          downloadEnabled: false,
          uploadEnabled: true,
          maxUploadBytes: null,
        });
      }
      if (input === "/api/v1/auth" && init?.method === "POST") return new Response(null, { status: 204 });
      if (input === "/api/v1/uploads" && init?.method === "POST") {
        createCalls += 1;
        return json({ uploadId: `upload-${createCalls}`, offset: 0, totalBytes: 4, chunkSize: 4, serviceId: "dienst", lastModified: 1 });
      }
      throw new Error(`Unerwartete Anfrage: ${String(input)}`);
    });

    render(<App />);
    const picker = await screen.findByLabelText(/Dateien auswählen/);
    await userEvent.upload(picker, new File(["AAAA"], "erste.bin", { lastModified: 1 }));
    await waitFor(() => expect(sends).toBe(1));
    await userEvent.upload(picker, new File(["BBBB"], "zweite.bin", { lastModified: 2 }));
    await act(async () => failFirst());

    const code = await screen.findByLabelText("Achtstelliger Zugangscode");
    await userEvent.type(code, "12345678");
    fireEvent.submit(code.closest("form")!);
    expect(await screen.findByText("erste.bin")).toBeTruthy();
    expect(screen.getByText(/Die Sitzung wurde unterbrochen/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Ausblenden" }));
    expect(screen.queryByText(/Die Sitzung wurde unterbrochen/)).toBeNull();
    const second = (await screen.findByText("zweite.bin")).closest("article")!;
    expect(second.querySelector(".upload-state")?.textContent).toBe("Wartet");
    await waitFor(() => expect(createCalls).toBe(2));
    const bodies = fetchMock.mock.calls
      .filter(([url, init]) => url === "/api/v1/uploads" && init?.method === "POST")
      .map(([, init]) => JSON.parse(String(init?.body)) as { name: string; clientToken: string });
    expect(bodies[0].name).toBe("erste.bin");
    expect(bodies[1].name).toBe("erste.bin");
    expect(bodies[1].clientToken).toBe(bodies[0].clientToken);
  });
});
