import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { AegisI18nProvider } from "@aegis/ui/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  MissionIssueDialog,
  useListIssuesByMission,
} from "../../../features/mission";
import { queryKeys } from "../../../shared/query";
import type { IssueViewResponse, MissionViewResponse } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

afterEach(() => cleanup());

const openedIssue: IssueViewResponse = {
  id: 1,
  missionId: 10,
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

const sampleMission: MissionViewResponse = {
  id: 10,
  projectCode: "alpha",
  missionKind: "crf",
  missionCode: "AE",
  assignees: [],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

function renderDialog(
  props: Partial<React.ComponentProps<typeof MissionIssueDialog>> = {},
) {
  const onClose = vi.fn();
  const onCreate = vi.fn();
  const onPatchState = vi.fn();
  const onUpdateDescription = vi.fn();
  const onAppendComment = vi.fn();

  const utils = render(
    <AegisI18nProvider>
      <MissionIssueDialog
        open
        scope={{ kind: "form" }}
        mission={sampleMission}
        issues={[]}
        canCreate={true}
        canActOnIssue={true}
        canComment={true}
        createPending={false}
        createError={null}
        onCreate={onCreate}
        patchPending={false}
        patchError={null}
        onPatchState={onPatchState}
        updateDescPending={false}
        updateDescError={null}
        onUpdateDescription={onUpdateDescription}
        commentPending={false}
        commentError={null}
        onAppendComment={onAppendComment}
        onClose={onClose}
        {...props}
      />
    </AegisI18nProvider>,
  );
  return {
    onClose,
    onCreate,
    onPatchState,
    onUpdateDescription,
    onAppendComment,
    ...utils,
  };
}

describe("MissionIssueDialog (shell)", () => {
  it("renders the title with the scope label", () => {
    renderDialog();
    expect(screen.getByText(/Mission issues/i)).toBeInTheDocument();
  });

  it("shows an empty Alert when there are no issues", () => {
    renderDialog();
    expect(screen.getByText(/No issues yet/i)).toBeInTheDocument();
  });

  it("renders a table with one row per issue", () => {
    renderDialog({ issues: [openedIssue, closedIssue] });
    // 2 data rows + 1 header row
    const rows = screen.getAllByRole("row");
    expect(rows.length).toBeGreaterThanOrEqual(3);
    // both issues have issuer "carol" — use getAllByText
    expect(screen.getAllByText("carol").length).toBeGreaterThanOrEqual(2);
    expect(
      screen.getAllByText(/missing CRF row in AE/i).length,
    ).toBeGreaterThanOrEqual(1);
  });

  it("calls onClose when the Cancel button is clicked", () => {
    const { onClose } = renderDialog();
    fireEvent.click(screen.getByRole("button", { name: /Cancel|Close/i }));
    expect(onClose).toHaveBeenCalled();
  });
});