import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useHealthz, useIsLoggedIn } from "../../data/bootstrap";
import { queryKeys } from "../../data/queryKeys";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => {
  cleanup();
});

function Probe({ enabled }: { enabled?: boolean }) {
  const health = useHealthz({ enabled });
  const status = useIsLoggedIn({ enabled });
  return (
    <>
      <button onClick={() => void health.refetch()}>refetch-health</button>
      <button onClick={() => void status.refetch()}>refetch-status</button>
      <span data-testid="health-data">{health.data ?? "none"}</span>
      <span data-testid="health-pending">{health.isPending ? "yes" : "no"}</span>
      <span data-testid="health-error-kind">{health.error?.kind ?? "none"}</span>
      <span data-testid="status-data">{String(status.data ?? "none")}</span>
    </>
  );
}

describe("useHealthz", () => {
  it("does not fetch on mount when enabled defaults to false", async () => {
    mockCommands({ healthz: () => "ok" });
    renderWithQueryClient(<Probe />);
    // Allow one tick for any spurious mount-time fetch.
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("fetches exactly once per manual refetch()", async () => {
    mockCommands({ healthz: () => "ok" });
    renderWithQueryClient(<Probe />);
    await screen.getByRole("button", { name: "refetch-health" }).click();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1);
      expect(invoke).toHaveBeenCalledWith("healthz");
    });
    expect(screen.getByTestId("health-data").textContent).toBe("ok");
  });

  it("propagates the thrown ApiError on refetch failure", async () => {
    mockCommands({
      healthz: () => {
        throw { kind: "network", message: "no route to host" };
      },
    });
    renderWithQueryClient(<Probe />);
    await screen.getByRole("button", { name: "refetch-health" }).click();
    await waitFor(() => {
      expect(screen.getByTestId("health-error-kind").textContent).toBe(
        "network",
      );
    });
  });

  it("treats cached data as immediately stale (staleTime: 0)", async () => {
    mockCommands({ healthz: () => "ok" });
    function AlwaysOn() {
      useHealthz({ enabled: true });
      return null;
    }
    const utils = renderWithQueryClient(<AlwaysOn />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    utils.unmount();

    // A second mount must trigger another fetch — the cached value is
    // stale (`staleTime: 0`), so `useQuery` never serves it from cache.
    // `dataUpdatedAt` confirms the first fetch is recorded.
    const client = utils.client;
    expect(
      client.getQueryState(queryKeys.bootstrap.health())?.dataUpdatedAt,
    ).toBeGreaterThan(0);

    renderWithQueryClient(<AlwaysOn />, { client });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});

describe("useIsLoggedIn", () => {
  it("does not fetch on mount when enabled defaults to false", async () => {
    mockCommands({ is_logged_in: () => true });
    renderWithQueryClient(<Probe />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("returns the boolean payload via manual refetch()", async () => {
    mockCommands({ is_logged_in: () => false });
    renderWithQueryClient(<Probe />);
    await screen.getByRole("button", { name: "refetch-status" }).click();
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("is_logged_in");
      expect(screen.getByTestId("status-data").textContent).toBe("false");
    });
  });

  it("treats cached data as immediately stale (staleTime: 0)", async () => {
    mockCommands({ is_logged_in: () => true });
    function AlwaysOn() {
      useIsLoggedIn({ enabled: true });
      return null;
    }
    const utils = renderWithQueryClient(<AlwaysOn />);
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    utils.unmount();

    const client = utils.client;
    expect(
      client.getQueryState(queryKeys.auth.loginStatus())?.dataUpdatedAt,
    ).toBeGreaterThan(0);

    renderWithQueryClient(<AlwaysOn />, { client });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});