import "@testing-library/jest-dom/vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import type { CrfForm } from "../../../shared/api";
import { computeNewFullOrder } from "../../../features/crf/pages/CrfFormListPage";
import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function renderPage(initialEntries: string[]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("CrfFormListPage", () => {
  it("renders the heading + one form row from the mocked backend", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "u",
        name: "U",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_crf_versions: () => ({
        versions: [{ id: 7, projectCode: "abc", name: "v1" }],
      }),
      list_crf_forms_by_version: () => ({
        forms: [
          {
            id: 11,
            versionId: 7,
            code: "AE",
            name: "Adverse Events",
            order: 0,
            notSubmitted: false,
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ],
      }),
      list_missions_by_project: () => [],
      get_project_by_code: () => ({
        id: 1,
        code: "abc",
        description: "",
        members: { leaders: [], workers: [] },
        unblindMembers: { leaders: [], workers: [] },
        tags: [],
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
    });

    renderPage(["/project/abc/crf?versionId=7"]);

    expect(
      await screen.findByRole("heading", { name: /CRF Form List/i }),
    ).toBeInTheDocument();

    expect(await screen.findByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("AE")).toBeInTheDocument();
  });

  it("renders the assignee's display name (resolved via useListUsers) on the chip", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "u",
        name: "U",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_crf_versions: () => ({
        versions: [{ id: 7, projectCode: "abc", name: "v1" }],
      }),
      list_crf_forms_by_version: () => ({
        forms: [
          {
            id: 11,
            versionId: 7,
            code: "AE",
            name: "Adverse Events",
            order: 0,
            notSubmitted: false,
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ],
      }),
      list_missions_by_project: () => [
        {
          id: 1,
          projectCode: "abc",
          missionKind: "crf",
          missionCode: "AE",
          assignees: [
            {
              id: 99,
              userCode: "carol",
              role: "qc",
              createdAt: "",
              updatedAt: "",
            },
          ],
          createdAt: "",
          updatedAt: "",
        },
      ],
      list_users: () => [
        {
          id: 2,
          code: "carol",
          name: "Carol Lin",
          role: "worker",
          active: true,
          createdAt: "",
          updatedAt: "",
        },
      ],
      get_project_by_code: () => ({
        id: 1,
        code: "abc",
        description: "",
        members: { leaders: [], workers: [] },
        unblindMembers: { leaders: [], workers: [] },
        tags: [],
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
    });

    renderPage(["/project/abc/crf?versionId=7"]);

    // The chip label is `name · role`; wait for the post-fetch
    // state since `useListUsers` resolves asynchronously.
    await waitFor(() =>
      expect(screen.getByText(/^Carol Lin · /)).toBeInTheDocument(),
    );
  });

  it("falls back to userCode on the chip when list_users returns an empty list", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "u",
        name: "U",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      list_crf_versions: () => ({
        versions: [{ id: 7, projectCode: "abc", name: "v1" }],
      }),
      list_crf_forms_by_version: () => ({
        forms: [
          {
            id: 11,
            versionId: 7,
            code: "AE",
            name: "Adverse Events",
            order: 0,
            notSubmitted: false,
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-01T00:00:00Z",
          },
        ],
      }),
      list_missions_by_project: () => [
        {
          id: 1,
          projectCode: "abc",
          missionKind: "crf",
          missionCode: "AE",
          assignees: [
            {
              id: 99,
              userCode: "carol",
              role: "qc",
              createdAt: "",
              updatedAt: "",
            },
          ],
          createdAt: "",
          updatedAt: "",
        },
      ],
      list_users: () => [],
      get_project_by_code: () => ({
        id: 1,
        code: "abc",
        description: "",
        members: { leaders: [], workers: [] },
        unblindMembers: { leaders: [], workers: [] },
        tags: [],
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
    });

    renderPage(["/project/abc/crf?versionId=7"]);

    // The chip falls back to the userCode when `list_users` returns
    // no rows. Use `findByText` (not `getByText`) since the chip
    // only appears once both `list_missions_by_project` and
    // `list_crf_forms_by_version` resolve.
    expect(await screen.findByText(/^carol · /)).toBeInTheDocument();
  });
});

describe("computeNewFullOrder", () => {
  const row = (id: number, code: string): CrfForm => ({
    id,
    versionId: 7,
    code,
    name: `Form ${code}`,
    order: id,
    notSubmitted: false,
    createdAt: "",
    updatedAt: "",
  });

  it("returns an empty array when allRows is empty", () => {
    expect(computeNewFullOrder([], [], [])).toEqual([]);
  });

  it("returns newVisibleIds unchanged when filteredRows equals allRows", () => {
    const allRows = [row(1, "AE"), row(2, "VS"), row(3, "LB")];
    const newOrder = computeNewFullOrder(allRows, [3, 1, 2], allRows);
    expect(newOrder).toEqual([3, 1, 2]);
  });

  it("splices the new visible order into the original full order, keeping hidden rows at their original slots", () => {
    // full = [A, B, C, D, E]; visible = [A, C, E]; newVisible = [E, A, C]
    // expected full = [E, B, A, D, C]
    const allRows = [
      row(1, "A"),
      row(2, "B"),
      row(3, "C"),
      row(4, "D"),
      row(5, "E"),
    ];
    const visibleRows = [allRows[0]!, allRows[2]!, allRows[4]!];
    expect(computeNewFullOrder(allRows, [5, 1, 3], visibleRows)).toEqual([
      5, 2, 1, 4, 3,
    ]);
  });

  it("falls back to the original id at a visible slot when newVisibleIds runs short", () => {
    // Defensive: cursor exhausted → preserve original id.
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C")];
    const visibleRows = [allRows[0]!, allRows[2]!];
    const out = computeNewFullOrder(allRows, [1], visibleRows);
    // The visible set in allRows is [A, C]; cursor consumes newVisibleIds[0]=1 → A;
    // then C slot — cursor >= 1 → fall back to original id 3.
    expect(out).toEqual([1, 2, 3]);
  });

  it("ignores a newVisibleIds tail beyond visibleRows.length", () => {
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C")];
    const visibleRows = [allRows[0]!];
    const out = computeNewFullOrder(allRows, [1, 99, 99], visibleRows);
    expect(out).toEqual([1, 2, 3]);
  });

  it("produces a full-length output regardless of input edge cases", () => {
    const allRows = [row(1, "A"), row(2, "B"), row(3, "C"), row(4, "D")];
    expect(computeNewFullOrder(allRows, [], [])).toEqual([1, 2, 3, 4]);
  });
});