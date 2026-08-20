import "@testing-library/jest-dom/vitest";
import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { useListCodeLists, useListCodeItems } from "./list";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";

function wrapper({ children }: { children: ReactNode }) {
  return <TestQueryProvider>{children}</TestQueryProvider>;
}

function pagedLists(codelists: unknown[], nextOffset?: number) {
  return { codelists, nextOffset };
}

function pagedItems(items: unknown[], nextOffset?: number) {
  return { items, nextOffset };
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});

describe("useListCodeLists", () => {
  it("returns the paged envelope and calls list_code_lists with the right args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(
      pagedLists([{ id: 1, code: "AE" }], undefined),
    );
    const { result } = renderHook(
      () => useListCodeLists(7, { fragment: "AE", offset: 0 }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.codelists).toEqual([{ id: 1, code: "AE" }]);
    expect(result.current.data?.nextOffset).toBeUndefined();
    expect(invoke).toHaveBeenCalledWith("list_code_lists", {
      versionId: 7,
      fragment: "AE",
      offset: 0,
      limit: 20,
    });
  });

  it("treats a whitespace fragment as no fragment", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(pagedLists([]));
    renderHook(() => useListCodeLists(7, { fragment: "   ", offset: 0 }), {
      wrapper,
    });
    await waitFor(() => expect(invoke).toHaveBeenCalled());
    expect(invoke).toHaveBeenCalledWith("list_code_lists", {
      versionId: 7,
      fragment: undefined,
      offset: 0,
      limit: 20,
    });
  });

  it("uses different query keys for different fragments", async () => {
    vi.mocked(invoke).mockResolvedValue(pagedLists([]));
    const { result: a } = renderHook(
      () => useListCodeLists(7, { fragment: "AE", offset: 0 }),
      { wrapper },
    );
    const { result: b } = renderHook(
      () => useListCodeLists(7, { fragment: "LB", offset: 0 }),
      { wrapper },
    );
    await waitFor(() => {
      expect(a.current.isSuccess).toBe(true);
      expect(b.current.isSuccess).toBe(true);
    });
    const fragmentArgs = vi
      .mocked(invoke)
      .mock.calls.map((c) => (c[1] as { fragment?: string }).fragment);
    expect(fragmentArgs).toContain("AE");
    expect(fragmentArgs).toContain("LB");
  });
});

describe("useListCodeItems", () => {
  it("returns the paged envelope and calls list_code_items with the right args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(
      pagedItems([{ id: 1, code: "Y" }], undefined),
    );
    const { result } = renderHook(
      () => useListCodeItems(11, { fragment: "Y", offset: 0 }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.items).toEqual([{ id: 1, code: "Y" }]);
    expect(invoke).toHaveBeenCalledWith("list_code_items", {
      codelistId: 11,
      fragment: "Y",
      offset: 0,
      limit: 20,
    });
  });

  it("treats a whitespace fragment as no fragment", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(pagedItems([]));
    renderHook(() => useListCodeItems(11, { fragment: "   ", offset: 0 }), {
      wrapper,
    });
    await waitFor(() => expect(invoke).toHaveBeenCalled());
    expect(invoke).toHaveBeenCalledWith("list_code_items", {
      codelistId: 11,
      fragment: undefined,
      offset: 0,
      limit: 20,
    });
  });
});
