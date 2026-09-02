import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CommandError } from "@dmdc/shared";
import { text } from "./i18n";

export { invoke, listen };

export function commandError(error: unknown): CommandError | null {
  if (!error || typeof error !== "object") return null;
  const candidate = error as Partial<CommandError>;
  if (typeof candidate.code !== "string" || typeof candidate.message !== "string") return null;
  return candidate as CommandError;
}

export function errorMessage(error: unknown): string {
  const structured = commandError(error);
  if (structured) return structured.message;
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return text.unknownError;
}
