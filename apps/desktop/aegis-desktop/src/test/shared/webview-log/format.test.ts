import { describe, expect, it } from "vitest";
import { formatConsoleArgs } from "../../../shared/webview-log/format";

describe("formatConsoleArgs", () => {
  it("returns the single string verbatim", () => {
    expect(formatConsoleArgs(["hello"])).toBe("hello");
  });

  it("joins multiple primitive args with a space", () => {
    expect(formatConsoleArgs(["a", 1, true])).toBe("a 1 true");
  });

  it("JSON-serialises plain objects", () => {
    expect(formatConsoleArgs([{ k: "v" }])).toBe('{"k":"v"}');
  });

  it("extracts Error name, message, and stack", () => {
    const err = new Error("boom");
    const out = formatConsoleArgs([err]);
    expect(out).toContain("Error: boom");
    expect(out).toContain(err.stack ?? ""); // stack presence is jsdom-dependent but the call must not throw
  });

  it("falls back to String() for unserialisable values", () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;
    const out = formatConsoleArgs([circular]);
    // JSON.stringify throws on circular; the fallback must not throw.
    expect(typeof out).toBe("string");
    expect(out.length).toBeGreaterThan(0);
  });

  it("returns an empty string for no args", () => {
    expect(formatConsoleArgs([])).toBe("");
  });
});