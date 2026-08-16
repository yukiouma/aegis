import { describe, expect, it } from "vitest";
import { shouldRedirectToBootstrap } from "../../../features/bootstrap/redirect";

describe("shouldRedirectToBootstrap", () => {
  it("returns true for the bare-root entry point", () => {
    expect(shouldRedirectToBootstrap("/")).toBe(true);
  });

  it("returns true for the Tauri index.html entry point", () => {
    expect(shouldRedirectToBootstrap("/index.html")).toBe(true);
  });

  it("returns false for /project/<code>/dashboard (workspace path)", () => {
    expect(shouldRedirectToBootstrap("/project/DEMO-001/dashboard")).toBe(
      false,
    );
  });

  it("returns false for /project/<code>/configuration (workspace path)", () => {
    expect(shouldRedirectToBootstrap("/project/DEMO-001/configuration")).toBe(
      false,
    );
  });

  it("returns false for /project/<code> (workspace index redirect)", () => {
    expect(shouldRedirectToBootstrap("/project/DEMO-001")).toBe(false);
  });

  it("returns false for /projects (main window paths)", () => {
    expect(shouldRedirectToBootstrap("/projects")).toBe(false);
  });

  it("returns false for /login", () => {
    expect(shouldRedirectToBootstrap("/login")).toBe(false);
  });
});