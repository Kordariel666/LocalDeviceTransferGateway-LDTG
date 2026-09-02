import type { ApiError } from "@dmdc/shared";

export class HttpError extends Error {
  constructor(public status: number, public code: string, message: string) {
    super(message);
  }
}

export async function api<T>(url: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(url, { credentials: "same-origin", ...init });
  if (!response.ok) {
    let body: ApiError = {
      code: `HTTP_${response.status}`,
      message: `HTTP ${response.status}`,
    };
    try {
      body = await response.json() as ApiError;
    } catch {
      // Text responses retain the HTTP fallback.
    }
    throw new HttpError(response.status, body.code, body.message);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}
