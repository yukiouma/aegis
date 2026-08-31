import { describe, expect, it } from "vitest";

import { errorMessage, httpCode, toApiError } from "../../../shared/api/error";

describe("toApiError", () => {
  it("passes through an object carrying a string `kind`", () => {
    const e = { kind: "refreshFailed" };
    expect(toApiError(e)).toEqual({ kind: "refreshFailed" });
  });

  it("wraps a thrown Error as a network error", () => {
    expect(toApiError(new Error("boom"))).toEqual({
      kind: "network",
      message: "Error: boom",
    });
  });

  it("wraps a plain string as a network error", () => {
    expect(toApiError("nope")).toEqual({ kind: "network", message: "nope" });
  });

  it("wraps null as a network error", () => {
    expect(toApiError(null)).toEqual({ kind: "network", message: "null" });
  });

  it("wraps an object with a non-string kind as a network error", () => {
    const e = toApiError({ kind: 42 });
    expect(e.kind).toBe("network");
  });
});

describe("httpCode", () => {
  it("returns the code for an http error", () => {
    expect(
      httpCode({ kind: "http", status: 404, code: "not_found", message: "no" }),
    ).toBe("not_found");
  });

  it("returns user_inactive for an inactive-account error", () => {
    expect(
      httpCode({ kind: "http", status: 403, code: "user_inactive", message: "x" }),
    ).toBe("user_inactive");
  });

  it("returns null for a network error", () => {
    expect(httpCode({ kind: "network", message: "dns" })).toBeNull();
  });

  it("returns null for a non-ApiError rejection", () => {
    expect(httpCode(new Error("boom"))).toBeNull();
  });
});

describe("errorMessage", () => {
  it("formats a network error", () => {
    expect(errorMessage({ kind: "network", message: "dns" })).toBe("dns");
  });

  it("formats an http error as code and message", () => {
    expect(
      errorMessage({ kind: "http", status: 401, code: "invalid_credentials", message: "bad" }),
    ).toBe("invalid_credentials: bad");
  });

  it("formats a refreshFailed error", () => {
    expect(errorMessage({ kind: "refreshFailed" })).toBe("refresh failed");
  });

  it("formats a notImplemented error using its detail", () => {
    expect(
      errorMessage({ kind: "notImplemented", detail: "requires Windows" }),
    ).toBe("requires Windows");
  });

  it("formats a store error", () => {
    expect(errorMessage({ kind: "store", message: "locked" })).toBe("locked");
  });

  it("formats a parse error using its message", () => {
    expect(
      errorMessage({
        kind: "parse",
        message: "3 validation error(s): form code empty; item BAD has kind 'text' but options are not allowed",
      }),
    ).toBe(
      "3 validation error(s): form code empty; item BAD has kind 'text' but options are not allowed",
    );
  });
});
