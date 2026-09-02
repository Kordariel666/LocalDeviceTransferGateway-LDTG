import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UploadQueueView } from "./UploadQueueView";
import { createUploadTiming, type UploadItem } from "./uploadQueue";

function item(id: string, state: UploadItem["state"], progress = 0): UploadItem {
  const file = new File([id], `${id}.bin`);
  return {
    id,
    file,
    state,
    progress,
    transferredBytes: Math.round(file.size * progress / 100),
    message: state,
    timing: createUploadTiming(),
  };
}

describe("UploadQueueView", () => {
  it("zeigt Batchfortschritt und verbindet alle Sammelaktionen", () => {
    const onPauseAll = vi.fn();
    const onResumeAll = vi.fn();
    const onRetryFailed = vi.fn();
    const onClearFinished = vi.fn();

    render(<UploadQueueView
      session={{ serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null }}
      uploads={[
        item("queued", "queued"),
        item("paused", "paused", 25),
        item("failed", "failed", 50),
        item("complete", "complete", 100),
      ]}
      summary={{ totalFiles: 4, finishedFiles: 1, totalBytes: 40, transferredBytes: 20, progress: 50 }}
      sessionNotice={null}
      onFiles={vi.fn()}
      onCancel={vi.fn()}
      onPause={vi.fn()}
      onResume={vi.fn()}
      onRetry={vi.fn()}
      onPauseAll={onPauseAll}
      onResumeAll={onResumeAll}
      onRetryFailed={onRetryFailed}
      onClearFinished={onClearFinished}
      onRemoveQueued={vi.fn()}
      onDismissSessionNotice={vi.fn()}
    />);

    expect(screen.getByText("1 von 4 Dateien erledigt · 20 B von 40 B")).toBeTruthy();
    expect(screen.getByRole("progressbar", { name: "Gesamtfortschritt aller ausgewählten Dateien" })).toHaveProperty("value", 50);
    for (const [name, callback] of [
      ["Alle pausieren", onPauseAll],
      ["Alle fortsetzen", onResumeAll],
      ["Fehlgeschlagene wiederholen", onRetryFailed],
      ["Erledigte entfernen", onClearFinished],
    ] as const) {
      fireEvent.click(screen.getByRole("button", { name }));
      expect(callback).toHaveBeenCalledOnce();
    }
  });

  it("lässt wartende Dateien entfernen und die Sitzungsmeldung ausblenden", () => {
    const queued = item("waiting", "queued");
    const onRemoveQueued = vi.fn();
    const onDismissSessionNotice = vi.fn();

    render(<UploadQueueView
      session={{ serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null }}
      uploads={[queued]}
      summary={{ totalFiles: 1, finishedFiles: 0, totalBytes: 7, transferredBytes: 0, progress: 0 }}
      sessionNotice="Die Sitzung wurde unterbrochen."
      onFiles={vi.fn()}
      onCancel={vi.fn()}
      onPause={vi.fn()}
      onResume={vi.fn()}
      onRetry={vi.fn()}
      onPauseAll={vi.fn()}
      onResumeAll={vi.fn()}
      onRetryFailed={vi.fn()}
      onClearFinished={vi.fn()}
      onRemoveQueued={onRemoveQueued}
      onDismissSessionNotice={onDismissSessionNotice}
    />);

    const upload = screen.getByText("waiting.bin").closest("article")!;
    fireEvent.click(within(upload).getByRole("button", { name: "Entfernen" }));
    expect(onRemoveQueued).toHaveBeenCalledWith(queued);
    fireEvent.click(screen.getByRole("button", { name: "Ausblenden" }));
    expect(onDismissSessionNotice).toHaveBeenCalledOnce();
  });

  it("kennzeichnet eine junge ETA trotz sichtbarer Geschwindigkeit als instabil", () => {
    const now = Date.now();
    const timed = {
      ...item("timed", "uploading", 40),
      file: new File([new Uint8Array(1_000)], "timed.bin"),
      transferredBytes: 400,
      timing: {
        ...createUploadTiming(),
        startedAt: now - 4_000,
        lastProgressAt: now - 1_000,
        sampledAt: now - 1_000,
        sampledBytes: 400,
        bytesPerSecond: 100,
        speedSampleCount: 2,
      },
    };

    render(<UploadQueueView
      session={{ serviceId: "dienst", csrfToken: "csrf", downloadEnabled: false, uploadEnabled: true, maxUploadBytes: null }}
      uploads={[timed]}
      summary={{ totalFiles: 1, finishedFiles: 0, totalBytes: 1_000, transferredBytes: 400, progress: 40 }}
      sessionNotice={null}
      onFiles={vi.fn()}
      onCancel={vi.fn()}
      onPause={vi.fn()}
      onResume={vi.fn()}
      onRetry={vi.fn()}
      onPauseAll={vi.fn()}
      onResumeAll={vi.fn()}
      onRetryFailed={vi.fn()}
      onClearFinished={vi.fn()}
      onRemoveQueued={vi.fn()}
      onDismissSessionNotice={vi.fn()}
    />);

    expect(screen.getByText("Geschwindigkeit 100 B/s")).toBeTruthy();
    expect(screen.getByText("Ungefähre Restzeit 6 s · noch instabil")).toBeTruthy();
  });
});
