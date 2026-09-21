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
        onPatchState={onPatchState}
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
      screen.queryByTestId("mission-issue-new-issue"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByLabelText(/Issue description/i),
    ).not.toBeInTheDocument();
  });

  it("shows the New Issue entry button by default", () => {
    renderDialog();
    expect(
      screen.getByTestId("mission-issue-new-issue"),
    ).toBeInTheDocument();
    expect(
      screen.queryByLabelText(/Issue description/i),
    ).not.toBeInTheDocument();
  });

  it("create submit is disabled while description is empty", () => {
    renderDialog();
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
    const submit = screen.getByRole("button", { name: /^Create$/i });
    expect(submit).toBeDisabled();
  });

  it("create submit is disabled when description is whitespace only", () => {
    renderDialog();
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "   " },
    });
    expect(screen.getByRole("button", { name: /^Create$/i })).toBeDisabled();
  });

  it("create submit is enabled when description is non-empty", () => {
    renderDialog();
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "missing CRF row in AE" },
    });
    expect(
      screen.getByRole("button", { name: /^Create$/i }),
    ).not.toBeDisabled();
  });

  it("clicking create invokes onCreate with the trimmed description", () => {
    const { onCreate } = renderDialog();
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "  missing row  " },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Create$/i }));
    expect(onCreate).toHaveBeenCalledWith("missing row");
  });

  it("submitting collapses the form back to the New Issue button and clears the draft", () => {
    const { onCreate } = renderDialog();
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "draft" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Create$/i }));
    expect(onCreate).toHaveBeenCalledWith("draft");
    // Form should be hidden again; the New Issue button should reappear.
    expect(
      screen.queryByLabelText(/Issue description/i),
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId("mission-issue-new-issue"),
    ).toBeInTheDocument();
  });

  it("Cancel collapses the form back to the New Issue button without calling onCreate", () => {
    const { onCreate } = renderDialog();
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
    fireEvent.change(screen.getByLabelText(/Issue description/i), {
      target: { value: "draft" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Cancel$/i }));
    expect(onCreate).not.toHaveBeenCalled();
    expect(
      screen.queryByLabelText(/Issue description/i),
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId("mission-issue-new-issue"),
    ).toBeInTheDocument();
  });

  it("shows the create error inline below the description TextField", () => {
    renderDialog({
      createError: { kind: "network", message: "boom" } as never,
    });
    fireEvent.click(screen.getByTestId("mission-issue-new-issue"));
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

  it("expands a row to show comments + comment form", () => {
    renderDialog({ issues: [issueWithComment] });
    expect(screen.queryByText(/will fix by EOD/i)).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    expect(screen.getByText(/will fix by EOD/i)).toBeInTheDocument();
  });

  it("toggling the Switch off an opened issue calls onPatchState(id, closed)", () => {
    const { onPatchState } = renderDialog({ issues: [openedIssue] });
    const switchEl = screen.getByRole("switch", { name: /Opened/i });
    expect(switchEl).toBeChecked();
    fireEvent.click(switchEl);
    expect(onPatchState).toHaveBeenCalledWith(openedIssue.id, "closed");
  });

  it("toggling the Switch on a closed issue calls onPatchState(id, opened)", () => {
    const { onPatchState } = renderDialog({ issues: [closedIssue] });
    const switchEl = screen.getByRole("switch", { name: /Closed/i });
    expect(switchEl).not.toBeChecked();
    fireEvent.click(switchEl);
    expect(onPatchState).toHaveBeenCalledWith(closedIssue.id, "opened");
  });

  it("Switch is disabled while patchPending is true", () => {
    renderDialog({
      issues: [openedIssue],
      patchPending: true,
    });
    expect(screen.getByRole("switch", { name: /Opened/i })).toBeDisabled();
  });

  it("append comment: click Add Comment → type → Comment invokes onAppendComment with trimmed content", () => {
    const { onAppendComment } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    // Reveal the comment form via the entry button.
    fireEvent.click(
      screen.getByTestId("mission-issue-comment-add"),
    );
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "  hello  " },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Comment$/i }));
    expect(onAppendComment).toHaveBeenCalledWith(openedIssue.id, "hello");
  });

  it("Comment submit is disabled when comment is empty or whitespace", () => {
    renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    // Reveal the comment form via the entry button.
    fireEvent.click(
      screen.getByTestId("mission-issue-comment-add"),
    );
    expect(
      screen.getByRole("button", { name: /^Comment$/i }),
    ).toBeDisabled();
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "   " },
    });
    expect(
      screen.getByRole("button", { name: /^Comment$/i }),
    ).toBeDisabled();
  });

  it("disables the state Switch when canActOnIssue is false", () => {
    renderDialog({ canActOnIssue: false, issues: [openedIssue] });
    expect(screen.getByRole("switch", { name: /Opened/i })).toBeDisabled();
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

  it("closing and reopening the dialog resets all internal state", () => {
    const { rerender } = renderDialog({ issues: [openedIssue] });
    fireEvent.click(screen.getByRole("button", { name: /expand/i }));
    fireEvent.click(screen.getByRole("button", { name: /Add comment/i }));
    fireEvent.change(screen.getByPlaceholderText(/Type a comment/i), {
      target: { value: "in-flight draft" },
    });
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
          onPatchState={() => undefined}
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
          onPatchState={() => undefined}
          commentPending={false}
          commentError={null}
          onAppendComment={() => undefined}
          onClose={() => undefined}
        />
      </AegisI18nProvider>,
    );
    expect(
      screen.queryByPlaceholderText(/Type a comment/i),
    ).not.toBeInTheDocument();
  });
});