import { describe, expect, it } from "vitest";
import {
  estimateTransferTiming,
  formatBytes,
  formatDuration,
  formatRate,
} from "./presentation";

describe("gemeinsame Transferdarstellung", () => {
  it("formatiert Größen, Raten und Dauern kompakt", () => {
    expect(formatBytes(1536)).toBe("1,5 KiB");
    expect(formatRate(1024 ** 2)).toBe("1 MiB/s");
    expect(formatDuration(65)).toBe("1 min 5 s");
    expect(formatDuration(Number.NaN)).toBe("–");
  });

  it("kennzeichnet junge und veraltete ETA-Schätzungen als instabil", () => {
    const input = {
      startedAt: 0,
      lastProgressAt: 3_000,
      finishedAt: null,
      active: true,
      transferredBytes: 400,
      totalBytes: 1_000,
      bytesPerSecond: 100,
      speedSampleCount: 3,
    };
    expect(estimateTransferTiming({ ...input, speedSampleCount: 2 }, 4_000).etaQuality).toBe("unstable");
    expect(estimateTransferTiming(input, 4_000)).toEqual({
      durationSeconds: 4,
      remainingSeconds: 6,
      etaQuality: "stable",
    });
    expect(estimateTransferTiming(input, 9_000).etaQuality).toBe("unstable");
    expect(estimateTransferTiming({ ...input, bytesPerSecond: null }, 4_000).etaQuality).toBe("unknown");
  });
});
