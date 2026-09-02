export type EtaQuality = "unknown" | "unstable" | "stable";

export type TransferTimingInput = {
  startedAt: string | number | null;
  lastProgressAt: string | number | null;
  finishedAt: string | number | null;
  active: boolean;
  transferredBytes: number;
  totalBytes: number;
  bytesPerSecond: number | null;
  speedSampleCount: number;
};

export type TransferTimingEstimate = {
  durationSeconds: number | null;
  remainingSeconds: number | null;
  etaQuality: EtaQuality;
};

const MIN_STABLE_SAMPLES = 3;
const MIN_STABLE_SPAN_MS = 2_000;
const STALE_PROGRESS_MS = 5_000;

function timestamp(value: string | number | null): number | null {
  if (value === null) return null;
  const parsed = typeof value === "number" ? value : Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${new Intl.NumberFormat("de-DE", { maximumFractionDigits: index ? 1 : 0 }).format(value / 1024 ** index)} ${units[index]}`;
}

export function formatDateTime(value: string | number): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return new Intl.DateTimeFormat("de-DE", { dateStyle: "short", timeStyle: "short" }).format(date);
}

export function formatDuration(value: number): string {
  if (!Number.isFinite(value) || value < 0) return "–";
  if (value < 1) return "< 1 s";
  const seconds = Math.round(value);
  if (seconds < 60) return `${seconds} s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  if (minutes < 60) return remainingSeconds ? `${minutes} min ${remainingSeconds} s` : `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes ? `${hours} h ${remainingMinutes} min` : `${hours} h`;
}

export function formatRate(bytesPerSecond: number): string {
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond <= 0) return "–";
  return `${formatBytes(bytesPerSecond)}/s`;
}

export function estimateTransferTiming(
  input: TransferTimingInput,
  now = Date.now(),
): TransferTimingEstimate {
  const startedAt = timestamp(input.startedAt);
  const lastProgressAt = timestamp(input.lastProgressAt);
  const finishedAt = timestamp(input.finishedAt);
  const end = input.active ? now : finishedAt;
  const durationSeconds = startedAt !== null && end !== null
    ? Math.max(0, end - startedAt) / 1_000
    : null;

  const hasEstimate = input.active
    && Number.isFinite(input.totalBytes)
    && input.totalBytes > input.transferredBytes
    && input.bytesPerSecond !== null
    && Number.isFinite(input.bytesPerSecond)
    && input.bytesPerSecond > 0;
  if (!hasEstimate) {
    return { durationSeconds, remainingSeconds: null, etaQuality: "unknown" };
  }

  const remainingSeconds = Math.max(0, input.totalBytes - input.transferredBytes)
    / (input.bytesPerSecond as number);
  const measurementSpan = startedAt !== null && lastProgressAt !== null
    ? lastProgressAt - startedAt
    : 0;
  const progressAge = lastProgressAt === null ? Number.POSITIVE_INFINITY : Math.max(0, now - lastProgressAt);
  const stable = input.speedSampleCount >= MIN_STABLE_SAMPLES
    && measurementSpan >= MIN_STABLE_SPAN_MS
    && progressAge <= STALE_PROGRESS_MS;
  return {
    durationSeconds,
    remainingSeconds,
    etaQuality: stable ? "stable" : "unstable",
  };
}
