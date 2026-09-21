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
        resolveName={(code) => code}
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

  it("renders the resolved name in the reviewer cell when resolveName maps the issuer", () => {
    renderDialog({
      issues: [openedIssue],
      // Override the default identity resolver with a name table.
      resolveName: (code) =>
        code === "carol" ? "Carol Q. Reviewer" : code,
    });
    expect(screen.getByText("Carol Q. Reviewer")).toBeInTheDocument();
  });

  it("falls back to the user_code when resolveName returns it (e.g. user not in cache)", () => {
    renderDialog({
      issues: [openedIssue],
      // Default identity resolver.
    });
    expect(screen.getByText("carol")).toBeInTheDocument();
  });

  it("calls onClose when the Cancel button is clicked", () => {
    const { onClose } = renderDialog();
    fireEvent.click(screen.getByRole("button", { name: /Cancel|Close/i }));
    expect(onClose).toHaveBeenCalled();
  });
});

describe("MissionIssueDialog — create form", () => {
  it("hides the create form when canCreate is false", () => {
    renderDialog({ canCreate: false });
    expect(
      screen.queryByLabelText(/Issue description/i),
    ).not.toBeInTheDocument();
  });

  it("create submit is disabled while description is empty", () => {
    renderDialog();
    const submit = screen.getByRole("button", { name: /^Create$/i });
    expect(submit).toBeDisabled();
  });

  it("create submit is disabled when description is whitespace only", () => {
    renderDialog();
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "   " },
    });
    expect(screen.getByRole("button", { name: /^Create$/i })).toBeDisabled();
  });

  it("create submit is enabled when description is non-empty", () => {
    renderDialog();
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "missing CRF row in AE" },
    });
    expect(
      screen.getByRole("button", { name: /^Create$/i }),
    ).not.toBeDisabled();
  });

  it("clicking create invokes onCreate with the trimmed description", () => {
    const { onCreate } = renderDialog();
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "  missing row  " },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Create$/i }));
    expect(onCreate).toHaveBeenCalledWith("missing row");
  });

  it("shows the create error inline below the description TextField", () => {
    renderDialog({
      createError: { kind: "network", message: "boom" } as never,
    });
    expect(screen.getByText(/boom/)).toBeInTheDocument();
  });
});

describe("MissionIssueDialog — per-row actions", () => {
  const issueWithComment: IssueViewResponse = {
    ...openedIssue,
    id: 5,
    comments: [
      {
        user: "bob",
        content: "will fix by EOD",
        createdAt: "2026-01-01T00:00:00Z",
      },
    ],
  };

  it("expands a row to show comments + edit + close/reopen + comment form", () => {
    renderDialog({ issues: [issueWithComment] });
    expect(screen.queryByText(/will fix by EOD/i)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByText(/will fix by EOD/i)).toBeInTheDocument();
  });

  it("clicking Close on an opened issue calls onPatchState(id, closed)", () => {
    const { onPatchState } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(
      screen.getByTestId(`mission-issue-close-${openedIssue.id}`),
    );
    expect(onPatchState).toHaveBeenCalledWith(openedIssue.id, "closed");
  });

  it("clicking Reopen on a closed issue calls onPatchState(id, opened)", () => {
    const { onPatchState } = renderDialog({ issues: [closedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(
      screen.getByTestId(`mission-issue-reopen-${closedIssue.id}`),
    );
    expect(onPatchState).toHaveBeenCalledWith(closedIssue.id, "opened");
  });

  it("edit description: click Edit → TextField pre-filled → Save invokes onUpdateDescription", () => {
    const { onUpdateDescription } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /Edit description/i }));
    const ta = screen.getByDisplayValue(openedIssue.description);
    fireEvent.change(ta, { target: { value: "new description" } });
    fireEvent.click(screen.getByRole("button", { name: /^Save$/i }));
    expect(onUpdateDescription).toHaveBeenCalledWith(
      openedIssue.id,
      "new description",
    );
  });

  it("append comment: typing then Send invokes onAppendComment with trimmed content", () => {
    const { onAppendComment } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "  hello  " },
    });
    fireEvent.click(screen.getByRole("button", { name: /Add comment/i }));
    expect(onAppendComment).toHaveBeenCalledWith(openedIssue.id, "hello");
  });

  it("Send is disabled when comment is empty or whitespace", () => {
    renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(
      screen.getByRole("button", { name: /Add comment/i }),
    ).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "   " },
    });
    expect(
      screen.getByRole("button", { name: /Add comment/i }),
    ).toBeDisabled();
  });

  it("hides Close/Reopen + Edit description when canActOnIssue is false", () => {
    renderDialog({ canActOnIssue: false, issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    // The data-testid pin distinguishes the per-row state-flip button
    // from the dialog's footer Close button.
    expect(
      screen.queryByTestId(`mission-issue-close-${openedIssue.id}`),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Edit description/i }),
    ).not.toBeInTheDocument();
  });

  it("hides the comment form when canComment is false", () => {
    renderDialog({
      canActOnIssue: false,
      canComment: false,
      issues: [openedIssue],
    });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(
      screen.queryByPlaceholderText(/Type a comment/i),
    ).not.toBeInTheDocument();
  });

  it("shows the patchError inline above the Close/Reopen button when present", () => {
    renderDialog({
      patchError: {
        kind: "http",
        status: 403,
        code: "forbidden",
        message: "nope",
      } as never,
      issues: [openedIssue],
    });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByText(/forbidden: nope/i)).toBeInTheDocument();
  });

  it("editing description on row A does not affect row B", () => {
    const rowA: IssueViewResponse = openedIssue;
    const rowB: IssueViewResponse = { ...openedIssue, id: 99 };
    renderDialog({ issues: [rowA, rowB] });
    fireEvent.click(screen.getAllByRole("button", { name: /expand/i })[0]);
    fireEvent.click(screen.getByRole("button", { name: /Edit description/i }));
    fireEvent.change(screen.getByDisplayValue(rowA.description), {
      target: { value: "row A edit" },
    });
    // collapse row A; expand row B
    fireEvent.click(screen.getAllByRole("button", { name: /expand/i })[0]);
    fireEvent.click(screen.getAllByRole("button", { name: /expand/i })[1]);
    expect(screen.queryByDisplayValue("row A edit")).not.toBeInTheDocument();
  });

  it("closing and reopening the dialog resets all internal state", () => {
    const { rerender } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /Edit description/i }));
    // close
    rerender(
      <AegisI18nProvider>
        <MissionIssueDialog
          open={false}
          scope={{ kind: "form" }}
          mission={sampleMission}
          issues={[openedIssue]}
          resolveName={(c) => c}
          canCreate={true}
          canActOnIssue={true}
          canComment={true}
          createPending={false}
          createError={null}
          onCreate={() => undefined}
          patchPending={false}
          patchError={null}
          onPatchState={() => undefined}
          updateDescPending={false}
          updateDescError={null}
          onUpdateDescription={() => undefined}
          commentPending={false}
          commentError={null}
          onAppendComment={() => undefined}
          onClose={() => undefined}
        />
      </AegisI18nProvider>,
    );
    // reopen
    rerender(
      <AegisI18nProvider>
        <MissionIssueDialog
          open={true}
          scope={{ kind: "form" }}
          mission={sampleMission}
          issues={[openedIssue]}
          resolveName={(c) => c}
          canCreate={true}
          canActOnIssue={true}
          canComment={true}
          createPending={false}
          createError={null}
          onCreate={() => undefined}
          patchPending={false}
          patchError={null}
          onPatchState={() => undefined}
          updateDescPending={false}
          updateDescError={null}
          onUpdateDescription={() => undefined}
          commentPending={false}
          commentError={null}
          onAppendComment={() => undefined}
          onClose={() => undefined}
        />
      </AegisI18nProvider>,
    );
    expect(
      screen.queryByRole("button", { name: /Edit description/i }),
    ).not.toBeInTheDocument();
  });
});