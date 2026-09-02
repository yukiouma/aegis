import "@testing-library/jest-dom/vitest";
import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useUserNameMap } from "../../../features/user/data/list";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function Probe({ code }: { code: string }) {
  const resolve = useUserNameMap();
  return <div data-testid="out">{resolve(code)}</div>;
}

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("useUserNameMap", () => {
  it("returns the user's name when the userCode is in the list", async () => {
    mockCommands({
      list_users: () => [
        {
          id: 1,
          code: "alice",
          name: "Alice Wong",
          role: "admin",
          active: true,
          createdAt: "",
          updatedAt: "",
        },
      ],
    });
    const { findByTestId } = render(
      <TestQueryProvider>
        <Probe code="alice" />
      </TestQueryProvider>,
    );
    const out = await findByTestId("out");
    // `findByTestId` resolves once the element is in the DOM, which
    // happens before React Query's `list_users` fetch returns. Wait
    // for the post-fetch state — only then should the resolver see
    // the data.
    await waitFor(() => expect(out.textContent).toBe("Alice Wong"));
  });

  it("falls back to the userCode when the list is empty", async () => {
    mockCommands({ list_users: () => [] });
    const { findByTestId } = render(
      <TestQueryProvider>
        <Probe code="alice" />
      </TestQueryProvider>,
    );
    expect((await findByTestId("out")).textContent).toBe("alice");
  });

  it("falls back to the userCode when the userCode is not in the list", async () => {
    mockCommands({ list_users: () => [
      { id: 1, code: "alice", name: "Alice Wong", role: "admin",
        active: true, createdAt: "", updatedAt: "" },
    ] });
    const { findByTestId } = render(
      <TestQueryProvider>
        <Probe code="ghost" />
      </TestQueryProvider>,
    );
    const out = await findByTestId("out");
    await waitFor(() => expect(out.textContent).toBe("ghost"));
  });
});