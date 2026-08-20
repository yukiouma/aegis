import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

// Two SDTM versions so the dropdown has a non-trivial selection. The
// bug we're guarding against only manifests when the user picked the
// non-default version — if there's only one, the "reset" is invisible.
const sdtmV1 = {
  id: 1,
  kind: "sdtm" as const,
  name: "2026-01-01",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const sdtmV2 = {
  id: 2,
  kind: "sdtm" as const,
  name: "2026-03-27",
  createdAt: "2026-03-27T00:00:00Z",
  updatedAt: "2026-03-27T00:00:00Z",
};
const sdtmCodelist = {
  id: 100,
  versionId: 2,
  code: "AE",
  extensible: true,
  name: "Adverse Events",
  submissionValue: "AE",
  synonym: "Adverse Event",
  definition: "Any untoward medical occurrence...",
  nciPreferredTerm: "Adverse Event",
  createdAt: "2026-03-27T00:00:00Z",
  updatedAt: "2026-03-27T00:00:00Z",
};

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

beforeEach(() => {
  mockInvoke.mockReset();
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => cleanup());

function renderRoot(initialEntries: string[]) {
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

function setupMocks() {
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
    list_terminology_versions: () => [sdtmV1, sdtmV2],
    list_code_lists: (args) => {
      // versionId=2 is the one with a codelist in this fixture
      if (args && args.versionId === 2) {
        return { items: [sdtmCodelist], nextOffset: undefined };
      }
      return { items: [], nextOffset: undefined };
    },
    get_code_list_by_id: () => sdtmCodelist,
    list_code_items: () => ({ items: [], nextOffset: undefined }),
  });
}

describe("TerminologyPage — VersionDropdown persistence across navigation", () => {
  it("preserves the selected version after drilling into a code list and clicking back", async () => {
    setupMocks();
    const user = userEvent.setup();

    // Land on the SDTM list page with version 2 already in the URL.
    const { router } = await renderRoot(["/terminology/sdtm?versionId=2"]);

    // The dropdown reflects the URL on first render.
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/terminology/sdtm");
      expect(router.state.location.search.versionId).toBe(2);
    });
    // The Select displays the label of the chosen option.
    expect(screen.getByRole("combobox")).toHaveTextContent(/2026-03-27/);

    // Click into the code list via its row's "open" launch button.
    const openRow = await screen.findByRole("button", { name: `open ${sdtmCodelist.code}` });
    await user.click(openRow);
    await waitFor(() => {
      expect(router.state.location.pathname).toBe(
        "/terminology/sdtm/codelists/100",
      );
    });

    // Click the back arrow. The detail page's back nav must forward
    // `versionId` so the dropdown survives the round-trip.
    await user.click(screen.getByRole("button", { name: /back/i }));
    await waitFor(() => {
      expect(router.state.location.pathname).toBe("/terminology/sdtm");
      expect(router.state.location.search.versionId).toBe(2);
    });
    expect(screen.getByRole("combobox")).toHaveTextContent(/2026-03-27/);
  });
});