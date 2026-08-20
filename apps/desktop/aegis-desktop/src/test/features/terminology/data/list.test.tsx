import "@testing-library/jest-dom/vitest";
import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import { TestQueryProvider } from "../../../helpers/test-query-provider";
import { useListCodeLists, useListCodeItems } from "../../../../features/terminology/data/list";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";

function wrapper({ children }: { children: ReactNode }) {
  return <TestQueryProvider>{children}</TestQueryProvider>;
}

function pagedLists(codelists: unknown[], nextOffset?: number) {
  return { items: codelists, nextOffset };
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
  it("returns the first page and calls list_code_lists with the right args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(
      pagedLists([{ id: 1, code: "AE" }], undefined),
    );
    const { result } = renderHook(
      () => useListCodeLists(7, { fragment: "AE" }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.pages[0]?.items).toEqual([{ id: 1, code: "AE" }]);
    expect(result.current.data?.pages[0]?.nextOffset).toBeUndefined();
    expect(invoke).toHaveBeenCalledWith("list_code_lists", {
      versionId: 7,
      fragment: "AE",
      offset: 0,
      limit: 20,
    });
  });

  it("treats a whitespace fragment as no fragment", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(pagedLists([]));
    renderHook(() => useListCodeLists(7, { fragment: "   " }), {
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
      () => useListCodeLists(7, { fragment: "AE" }),
      { wrapper },
    );
    const { result: b } = renderHook(
      () => useListCodeLists(7, { fragment: "LB" }),
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

  it("appends the next page onto data.pages when fetchNextPage is called", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(pagedLists([{ id: 1 }, { id: 2 }], 20))
      .mockResolvedValueOnce(pagedLists([{ id: 3 }, { id: 4 }], undefined));
    const { result } = renderHook(
      () => useListCodeLists(7, { fragment: "" }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.pages).toHaveLength(1);
    expect(result.current.hasNextPage).toBe(true);

    await result.current.fetchNextPage();

    await waitFor(() =>
      expect(result.current.data?.pages).toHaveLength(2),
    );
    expect(result.current.data?.pages[0]?.items).toEqual([{ id: 1 }, { id: 2 }]);
    expect(result.current.data?.pages[1]?.items).toEqual([{ id: 3 }, { id: 4 }]);
    expect(result.current.hasNextPage).toBe(false);
    expect(invoke).toHaveBeenNthCalledWith(2, "list_code_lists", {
      versionId: 7,
      fragment: undefined,
      offset: 20,
      limit: 20,
    });
  });
});

describe("useListCodeItems", () => {
  it("returns the first page and calls list_code_items with the right args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(
      pagedItems([{ id: 1, code: "Y" }], undefined),
    );
    const { result } = renderHook(
      () => useListCodeItems(11, { fragment: "Y" }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.pages[0]?.items).toEqual([{ id: 1, code: "Y" }]);
    expect(invoke).toHaveBeenCalledWith("list_code_items", {
      codelistId: 11,
      fragment: "Y",
      offset: 0,
      limit: 20,
    });
  });

  it("treats a whitespace fragment as no fragment", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(pagedItems([]));
    renderHook(() => useListCodeItems(11, { fragment: "   " }), {
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

  it("appends the next page onto data.pages when fetchNextPage is called", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(pagedItems([{ id: 1 }, { id: 2 }], 20))
      .mockResolvedValueOnce(pagedItems([{ id: 3 }, { id: 4 }], undefined));
    const { result } = renderHook(
      () => useListCodeItems(11, { fragment: "" }),
      { wrapper },
    );
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data?.pages).toHaveLength(1);
    expect(result.current.hasNextPage).toBe(true);

    await result.current.fetchNextPage();

    await waitFor(() =>
      expect(result.current.data?.pages).toHaveLength(2),
    );
    expect(result.current.data?.pages.flatMap((p) => p.items)).toEqual([
      { id: 1 },
      { id: 2 },
      { id: 3 },
      { id: 4 },
    ]);
    expect(invoke).toHaveBeenNthCalledWith(2, "list_code_items", {
      codelistId: 11,
      fragment: undefined,
      offset: 20,
      limit: 20,
    });
  });
});