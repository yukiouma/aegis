import "@testing-library/jest-dom/vitest";
import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import { renderWithFullRouter } from "../../helpers/file-route-utils";

const domain = {
  id: 7,
  versionId: 5,
  name: "AE",
  category: "Events",
  descriptions: [
    {
      lang: "en",
      details: { description: "Adverse Events", structure: "One per AE" },
    },
  ],
  createdAt: "",
  updatedAt: "",
};

const variables = [
  {
    id: 1,
    domainId: 7,
    name: "AETERM",
    variableType: "Character",
    variableCore: "Req",
    variableRole: "Topic",
    variableSequence: 1,
    descriptions: [{ lang: "en", details: { label: "Term" } }],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    domainId: 7,
    name: "AESEV",
    variableType: "Character",
    variableCore: "Req",
    variableRole: "Record Qualifier",
    variableSequence: 2,
    descriptions: [{ lang: "en", details: { label: "Severity" } }],
    createdAt: "",
    updatedAt: "",
  },
];

function setupMocks() {
  mockInvoke.mockReset();
  mockCommands({
    is_logged_in: () => true,
    current_user: () => ({
      id: 1,
      code: "alice",
      name: "Alice",
      role: "admin",
      active: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    }),
    get_sdtm_domain_by_id: () => domain,
    list_sdtm_variables_by_domain: () => ({ variables }),
    create_sdtm_variable: (args?: Record<string, unknown>) => ({
      ...args,
      id: 99,
      createdAt: "2026-02-01T00:00:00Z",
      updatedAt: "2026-02-01T00:00:00Z",
    }),
    update_sdtm_variable: () => variables[0],
    delete_sdtm_variable: () => undefined,
  });
}

function renderPage(initial = "/domain-model/sdtm/7?lang=en") {
  return renderWithFullRouter({
    initialEntries: [initial],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("SdtmDomainDetail", () => {
  beforeEach(() => setupMocks());
  afterEach(() => cleanup());

  it("renders the domain header and variable rows", async () => {
    renderPage();
    expect(await screen.findByText("AE")).toBeInTheDocument();
    expect(await screen.findByText("AETERM")).toBeInTheDocument();
    expect(await screen.findByText("AESEV")).toBeInTheDocument();
  });

  it("filters variables by name OR label", async () => {
    renderPage();
    await screen.findByText("AETERM");
    const input = screen.getByLabelText(/Filter by name or label/i);
    await userEvent.type(input, "Severity");
    await waitFor(() => {
      expect(screen.queryByText("AETERM")).not.toBeInTheDocument();
      expect(screen.getByText("AESEV")).toBeInTheDocument();
    });
  });

  it("opens the variable create drawer with max+1 sequence", async () => {
    renderPage();
    await screen.findByText("AETERM");
    const headerCreate = screen.getByRole("button", {
      name: /create variable/i,
    });
    await userEvent.click(headerCreate);
    await userEvent.type(
      screen.getByRole("textbox", { name: /^name$/i }),
      "AETOX",
    );
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "create_sdtm_variable",
        expect.objectContaining({
          input: expect.objectContaining({
            domainId: 7,
            variableSequence: 3,
          }),
        }),
      );
    });
  });

  it("opens the variable delete dialog and removes the row on confirm", async () => {
    renderPage();
    await screen.findByText("AETERM");
    const deleteButtons = await screen.findAllByRole("button", {
      name: /delete variable/i,
    });
    await userEvent.click(deleteButtons[0]);
    await userEvent.click(
      await screen.findByRole("button", { name: /confirm/i }),
    );
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("delete_sdtm_variable", {
        id: 1,
      });
    });
  });

  it("does not PUT update_sdtm_variable on initial load", async () => {
    mockInvoke.mockReset();
    mockCommands({
      is_logged_in: () => true,
      current_user: () => ({
        id: 1,
        code: "alice",
        name: "Alice",
        role: "admin",
        active: true,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      }),
      get_sdtm_domain_by_id: () => domain,
      list_sdtm_variables_by_domain: () => ({ variables }),
      update_sdtm_variable: () => variables[0],
    });
    renderPage();
    await screen.findByText("AETERM");
    const updateCalls = mockInvoke.mock.calls.filter(
      (c) => c[0] === "update_sdtm_variable",
    );
    expect(updateCalls.length).toBe(0);
  });
});