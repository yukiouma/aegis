import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useListIssuesByMission } from "../../../features/mission";
import { queryKeys } from "../../../shared/query";
import type { IssueViewResponse } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

afterEach(() => cleanup());

const openedIssue: IssueViewResponse = {
  id: 1,
  missionId: 10,
  targetItem: null,
  issuer: "carol",
  description: "missing CRF row in AE",
  state: "opened",
  comments: [],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const closedIssue: IssueViewResponse = {
  ...openedIssue,
  id: 2,
  state: "closed",
};

function ListProbe({ missionId }: { missionId: number | null }) {
  const q = useListIssuesByMission(missionId);
  return (
    <span data-testid="result">{JSON.stringify(q.data ?? null)}</span>
  );
}

describe("useListIssuesByMission", () => {
  it("does not fetch when missionId is null", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    mockCommands({ list_issues_by_mission: () => [openedIssue] });
    renderWithQueryClient(<ListProbe missionId={null} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalledWith(
      "list_issues_by_mission",
      expect.anything(),
    );
  });

  it("invokes list_issues_by_mission with { missionId, state } and exposes the array", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    mockCommands({ list_issues_by_mission: () => [openedIssue, closedIssue] });
    renderWithQueryClient(<ListProbe missionId={10} />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_issues_by_mission", {
        missionId: 10,
        state: undefined,
      });
    });
    await waitFor(() => {
      expect(screen.getByTestId("result")).toHaveTextContent(
        JSON.stringify([openedIssue, closedIssue]),
      );
    });
  });

  it("uses queryKeys.mission.issuesByMission(missionId) as the query key", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
    mockCommands({ list_issues_by_mission: () => [openedIssue] });
    const { client } = renderWithQueryClient(<ListProbe missionId={10} />);
    await waitFor(() => {
      const cache = client.getQueryCache().getAll();
      expect(
        cache.some(
          (q) =>
            JSON.stringify(q.queryKey) ===
            JSON.stringify(queryKeys.mission.issuesByMission(10)),
        ),
      ).toBe(true);
    });
  });
});