import { invoke } from "@tauri-apps/api/core";
import type { Mock } from "vitest";

export type CommandHandlers = Record<
  string,
  (args?: Record<string, unknown>) => unknown
>;

/**
 * The mocked `invoke`. The importing test file must itself call
 * `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` — `vi.mock`
 * is hoisted per-file and cannot be applied from a helper module.
 */
export const mockInvoke = invoke as unknown as Mock;

/**
 * Dispatch mocked Tauri commands by name rather than by call order, so a
 * test does not break when an unrelated command joins a page's startup
 * sequence. A handler that throws becomes a rejected promise, which is how
 * a genuinely failing command surfaces to the caller.
 */
export function mockCommands(handlers: CommandHandlers): void {
  mockInvoke.mockImplementation(
    (cmd: string, args?: Record<string, unknown>) => {
      const handler = handlers[cmd];
      if (!handler) {
        return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
      }
      try {
        return Promise.resolve(handler(args));
      } catch (e) {
        return Promise.reject(e);
      }
    },
  );
}

/** Build the `ApiError` shape Tauri rejects an HTTP failure with. */
export function httpError(status: number, code: string, message = "err") {
  return { kind: "http", status, code, message };
}
