import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock the store BEFORE importing the module under test so the
// singleton store handle is constructed against the mock loader.
const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () =>
    Promise.resolve({
      get: (k: string) => Promise.resolve(store.get(k)),
      set: (k: string, v: unknown) => {
        store.set(k, v);
        return Promise.resolve();
      },
      delete: (k: string) => {
        store.delete(k);
        return Promise.resolve();
      },
      save: () => Promise.resolve(),
    }),
}));

import {
  readUpdateDeferral,
  writeUpdateDeferral,
} from "../../../features/settings";

beforeEach(() => store.clear());

describe("the update deferral", () => {
  it("round-trips the deferred version through the settings store", async () => {
    expect(await readUpdateDeferral()).toBeNull();
    await writeUpdateDeferral("0.1.1");
    expect(await readUpdateDeferral()).toBe("0.1.1");
    await writeUpdateDeferral(null);
    expect(await readUpdateDeferral()).toBeNull();
  });

  it("writes the deferral under a namespaced key", async () => {
    await writeUpdateDeferral("0.1.1");
    expect(store.get("update.deferredVersion")).toBe("0.1.1");
  });
});
