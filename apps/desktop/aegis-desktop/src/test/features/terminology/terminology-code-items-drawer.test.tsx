import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import { TerminologyCodeItemsDrawer } from
  "../../../features/terminology/components/TerminologyCodeItemsDrawer";

const sdtmVersion = {
  id: 5,
  kind: "sdtm",
  name: "2024-06-28",
  createdAt: "",
  updatedAt: "",
};

const adamVersion = {
  id: 9,
  kind: "adam",
  name: "2024-09-30",
  createdAt: "",
  updatedAt: "",
};

const targetCodelist = {
  id: 11,
  versionId: 5,
  code: "C66741",
  extensible: true,
  name: "VSTESTCD",
  submissionValue: "VSTESTCD",
  synonym: "",
  definition: "",
  nciPreferredTerm: "",
  createdAt: "",
  updatedAt: "",
};

const decoyCodelist = {
  ...targetCodelist,
  id: 12,
  name: "Other",
  definition: "mentions VSTESTCD in its definition",
};

const codeItem = {
  id: 21,
  codelistId: 11,
  versionId: 5,
  code: "C174106",
  submissionValue: "PASS",
  synonym: "",
  definition: "",
  nciPreferredTerm: "",
  createdAt: "",
  updatedAt: "",
};

function projectWithTerminology(versionId: number, versionName: string) {
  return {
    id: 1,
    code: "DEMO-001",
    description: "",
    members: { users: [], groups: [] },
    configurations: {
      language: "en",
      tags: [],
      sdtmig: null,
      sdtmTerminology: { versionId, versionName },
    },
    active: true,
    createdAt: "",
    updatedAt: "",
  };
}

function setupMocks(overrides?: {
  versions?: unknown[];
  codeLists?: { items: unknown[] };
  codeItems?: { items: unknown[] };
}) {
  mockInvoke.mockReset();
  mockCommands({
    list_terminology_versions: () =>
      overrides?.versions ?? [sdtmVersion, adamVersion],
    list_code_lists: () =>
      overrides?.codeLists ?? { items: [decoyCodelist, targetCodelist] },
    list_code_items: () => overrides?.codeItems ?? { items: [codeItem] },
  });
}

function renderDrawer(props?: {
  open?: boolean;
  code?: string | null;
  projectCode?: string | null;
}) {
  return render(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <TerminologyCodeItemsDrawer
            open={props?.open ?? true}
            code={props?.code ?? "VSTESTCD"}
            projectCode={props?.projectCode ?? null}
            onClose={() => {}}
          />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("TerminologyCodeItemsDrawer", () => {
  afterEach(() => cleanup());

  it("resolves the code against the latest SDTM version and lists the code list's items", async () => {
    setupMocks();
    renderDrawer();

    expect(await screen.findByText("PASS")).toBeInTheDocument();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_lists",
        expect.objectContaining({ versionId: 5, fragment: "VSTESTCD" }),
      );
    });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_items",
        expect.objectContaining({ codelistId: 11 }),
      );
    });
  });

  it("picks the code list whose name matches exactly, not a full-text hit", async () => {
    setupMocks();
    renderDrawer();
    expect(await screen.findByText("PASS")).toBeInTheDocument();
    // The decoy code list (id 12) also mentions the code in its definition;
    // only the exact name match (id 11) is used.
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter(
        (c) => c[0] === "list_code_items",
      );
      expect(
        calls.every(
          (c) => (c[1] as { codelistId: number }).codelistId === 11,
        ),
      ).toBe(true);
    });
  });

  it("shows a not-found state when no code list matches", async () => {
    setupMocks({ codeLists: { items: [decoyCodelist] } });
    renderDrawer();
    expect(
      await screen.findByText(/no code list named VSTESTCD was found/i),
    ).toBeInTheDocument();
  });

  it("shows an error state when the terminology version query fails", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => {
        throw { kind: "network", message: "server unreachable" };
      },
      list_code_lists: () => ({ items: [targetCodelist] }),
      list_code_items: () => ({ items: [codeItem] }),
    });
    renderDrawer();
    expect(
      await screen.findByText(/failed to load the terminology versions/i),
    ).toBeInTheDocument();
    // A failed request is not the same as "nothing is imported".
    expect(
      screen.queryByText(/no sdtm terminology version has been imported/i),
    ).not.toBeInTheDocument();
  });

  it("shows a spinner while the terminology version query is in flight", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => new Promise(() => {}),
      list_code_lists: () => ({ items: [targetCodelist] }),
      list_code_items: () => ({ items: [codeItem] }),
    });
    renderDrawer();
    await waitFor(() => {
      expect(screen.getByRole("progressbar")).toBeInTheDocument();
    });
    expect(
      screen.queryByText(/no sdtm terminology version has been imported/i),
    ).not.toBeInTheDocument();
  });

  it("retries only the failing query", async () => {
    mockInvoke.mockReset();
    let codeListAttempts = 0;
    mockCommands({
      list_terminology_versions: () => [sdtmVersion],
      list_code_lists: () => {
        codeListAttempts += 1;
        if (codeListAttempts === 1) {
          throw { kind: "http", status: 500, code: "boom", message: "boom" };
        }
        return { items: [targetCodelist] };
      },
      list_code_items: () => ({ items: [codeItem] }),
    });
    renderDrawer();
    const retry = await screen.findByRole("button", { name: /retry/i });
    await userEvent.click(retry);
    await waitFor(() => {
      expect(codeListAttempts).toBe(2);
    });
    // The code-item query must never be fired without a resolved code
    // list — an unfiltered request would ask the server for an
    // arbitrary page of code items.
    const itemCalls = mockInvoke.mock.calls.filter(
      (c) => c[0] === "list_code_items",
    );
    expect(
      itemCalls.every(
        (c) => (c[1] as { codelistId: number }).codelistId === 11,
      ),
    ).toBe(true);
  });

  it("resolves a code list whose name only matches case-insensitively", async () => {
    // The search is case-insensitive, so `vstestcd` still finds the code
    // list; the resolution must accept it rather than reporting a miss.
    setupMocks();
    renderDrawer({ code: "vstestcd" });
    expect(await screen.findByText("PASS")).toBeInTheDocument();
  });

  it("shows an error state when the code list lookup fails", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => [sdtmVersion],
      list_code_lists: () => {
        throw {
          kind: "http",
          status: 400,
          code: "invalid_fragment",
          message: "bad fragment",
        };
      },
      list_code_items: () => ({ items: [] }),
    });
    renderDrawer();
    expect(await screen.findByText(/failed to load/i)).toBeInTheDocument();
  });

  it("shows a distinct state when no SDTM terminology version exists", async () => {
    setupMocks({ versions: [adamVersion] });
    renderDrawer();
    expect(
      await screen.findByText(/no sdtm terminology version/i),
    ).toBeInTheDocument();
  });

  it("renders the items read-only — no create, edit, or delete affordances", async () => {
    setupMocks();
    renderDrawer();
    await screen.findByText("PASS");
    expect(
      screen.queryByRole("button", { name: /create code item/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /edit code item/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /delete C174106/i }),
    ).not.toBeInTheDocument();
  });

  it("does not query while the drawer is closed", async () => {
    setupMocks();
    renderDrawer({ open: false });
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter(
        (c) => c[0] === "list_code_lists",
      );
      expect(calls.length).toBe(0);
    });
  });

  it("resolves against the terminology version the project configures", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => [
        sdtmVersion,
        { ...sdtmVersion, id: 7, name: "2024-09-27" },
        adamVersion,
      ],
      get_project_by_code: () => projectWithTerminology(5, "2024-06-28"),
      list_code_lists: () => ({ items: [targetCodelist] }),
      list_code_items: () => ({ items: [codeItem] }),
    });
    renderDrawer({ projectCode: "DEMO-001" });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_lists",
        expect.objectContaining({ versionId: 5 }),
      );
    });
    // Not the most recently imported SDTM release (id 7).
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "list_code_lists",
      expect.objectContaining({ versionId: 7 }),
    );
  });

  it("resolves a stale configured version id by name", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => [
        sdtmVersion,
        { ...sdtmVersion, id: 7, name: "2024-09-27" },
      ],
      // The stored id no longer exists; the stored name still does.
      get_project_by_code: () => projectWithTerminology(99, "2024-06-28"),
      list_code_lists: () => ({ items: [targetCodelist] }),
      list_code_items: () => ({ items: [codeItem] }),
    });
    renderDrawer({ projectCode: "DEMO-001" });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_lists",
        expect.objectContaining({ versionId: 5 }),
      );
    });
    // Not the most recently imported SDTM release (id 7).
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "list_code_lists",
      expect.objectContaining({ versionId: 7 }),
    );
  });

  it("falls back to the newest SDTM release when the project configures none", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => [
        sdtmVersion,
        { ...sdtmVersion, id: 7, name: "2024-09-27" },
        adamVersion,
      ],
      get_project_by_code: () => ({
        ...projectWithTerminology(5, "2024-06-28"),
        configurations: { language: "en", tags: [], sdtmTerminology: null },
      }),
      list_code_lists: () => ({ items: [targetCodelist] }),
      list_code_items: () => ({ items: [codeItem] }),
    });
    renderDrawer({ projectCode: "DEMO-001" });
    await waitFor(() => {
      // The project was consulted…
      expect(mockInvoke).toHaveBeenCalledWith("get_project_by_code", {
        code: "DEMO-001",
      });
    });
    await waitFor(() => {
      // …and the newest SDTM release was used.
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_lists",
        expect.objectContaining({ versionId: 7 }),
      );
    });
  });
});
