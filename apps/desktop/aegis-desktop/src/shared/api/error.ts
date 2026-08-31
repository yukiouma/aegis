// Tauri rejects a command with the serialized `ApiError` object, typed as
// `unknown` on the JS side. These helpers narrow that value once so pages
// do not each re-implement the `kind === "http"` dance.

import type { ApiError } from "./types";

/**
 * Narrow an unknown rejection value to `ApiError`. Anything that is not a
 * tagged `ApiError` (a thrown JS `Error`, a string, null) degrades to a
 * `network` error carrying its stringified form, so callers always get a
 * usable message and `httpCode` returns null for it.
 */
export function toApiError(e: unknown): ApiError {
  if (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    typeof (e as { kind: unknown }).kind === "string"
  ) {
    return e as ApiError;
  }
  return { kind: "network", message: String(e) };
}

/**
 * The server's stable, machine-readable error token (`not_found`,
 * `user_inactive`, `invalid_credentials`, ...) for HTTP errors, or null
 * for every other failure kind.
 */
export function httpCode(e: unknown): string | null {
  const err = toApiError(e);
  return err.kind === "http" ? err.code : null;
}

/** A human-readable one-line rendering of any failure, for the splash log. */
export function errorMessage(e: unknown): string {
  const err = toApiError(e);
  switch (err.kind) {
    case "network":
      return err.message;
    case "http":
      return `${err.code}: ${err.message}`;
    case "refreshFailed":
      return "refresh failed";
    case "notImplemented":
      return err.detail;
    case "store":
      return err.message;
    case "parse":
      return err.message;
  }
}
