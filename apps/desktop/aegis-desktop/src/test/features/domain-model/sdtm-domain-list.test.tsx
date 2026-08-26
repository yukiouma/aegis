import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

const versions = [
  { id: 1, name: "v1", createdAt: "", updatedAt: "" },
  { id: 2, name: "v2", createdAt: "", updatedAt: "" },
];

const domains = [
  {
    id: 1,
    versionId: 1,
    name: "AE",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Adverse Events", structure: "One per AE" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    versionId: 1,
    name: "DM",
    category: "Special Purpose",
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
];

beforeEach(() => {
  mockInvoke.mockReset();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
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
    list_sdtm_versions: () => ({ versions }),
    list_sdtm_domains_by_version: () => ({
      domains: [
        ...domains,
        {
          id: 99,
          versionId: 1,
          name: "ZZ",
          category: "Findings",
          descriptions: [
            { lang: "en", details: { description: "ZZ created", structure: "One per ZZ" } },
          ],
          createdAt: "",
          updatedAt: "",
        },
      ],
    }),
    create_sdtm_domain: () => ({
      id: 99,
      versionId: 1,
      name: "ZZ",
      category: "Findings",
      descriptions: [
        { lang: "en", details: { description: "ZZ created", structure: "One per ZZ" } },
      ],
      createdAt: "",
      updatedAt: "",
    }),
    delete_sdtm_domain: () => undefined,
  });
});

afterEach(() => cleanup());

function renderPage() {
  return renderWithFullRouter({
    initialEntries: ["/domain-model/sdtm"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("SdtmDomainList", () => {
  it("renders the rows from the data fetch", async () => {
    renderPage();
    expect(await screen.findByText("AE")).toBeInTheDocument();
    expect(screen.getByText("DM")).toBeInTheDocument();
    expect(screen.getByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("One per AE")).toBeInTheDocument();
  });

  it("filters by the search field", async () => {
    renderPage();
    const input = await screen.findByLabelText(/Filter by name or description/i);
    await userEvent.type(input, "DM");
    await waitFor(() => {
      expect(screen.queryByText("AE")).not.toBeInTheDocument();
    });
    expect(screen.getByText("DM")).toBeInTheDocument();
  });

  it("renders the delete icon for admin role", async () => {
    renderPage();
    expect(
      (await screen.findAllByRole("button", { name: /delete/i })).length,
    ).toBeGreaterThan(0);
  });

  it("shows the no-versions placeholder when no SDTM versions exist", async () => {
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
      list_sdtm_versions: () => ({ versions: [] }),
      list_sdtm_domains_by_version: () => ({ domains: [] }),
      delete_sdtm_domain: () => undefined,
    });
    renderPage();
    expect(
      await screen.findByText(/No SDTM versions exist yet/i),
    ).toBeInTheDocument();
  });

  it("opens the create drawer from the header + button and creates a new domain", async () => {
    renderPage();
    const createBtn = await screen.findByRole("button", {
      name: /create domain/i,
    });
    await userEvent.click(createBtn);
    const nameInput = await screen.findByRole("textbox", { name: /^code$/i });
    await userEvent.type(nameInput, "ZZ");
    await userEvent.click(
      screen.getByRole("button", { name: /^Description$/ }),
    );
    const descInput = await screen.findByRole("textbox", {
      name: /^description$/i,
    });
    await userEvent.type(descInput, "ZZ created");
    const structInput = screen.getByRole("textbox", { name: /^structure$/i });
    await userEvent.type(structInput, "One per ZZ");
    await userEvent.click(screen.getByRole("button", { name: /^Create$/ }));
    expect(await screen.findByText("ZZ created")).toBeInTheDocument();
    expect(screen.getByText("One per ZZ")).toBeInTheDocument();
  });
});