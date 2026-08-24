import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { renderInRouter } from "../../helpers/file-route-utils";
import { MetadataPage } from "../../../features/metadata";

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
  vi.unstubAllGlobals();
  vi.stubGlobal("localStorage", createMemoryStorage());
});
afterEach(() => {
  cleanup();
});

function renderMetadata() {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <MetadataPage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

function findCardByTitle(title: string): HTMLElement {
  // MUI's CardHeader renders its `title` as a <span> with class
  // `MuiCardHeader-title`. The closest `MuiCard-root` is the card itself.
  const titleEl = screen.getByText(title);
  const card = titleEl.closest(".MuiCard-root");
  if (!card) {
    throw new Error(`No MUI Card found containing "${title}"`);
  }
  return card as HTMLElement;
}

describe("MetadataPage", () => {
  it("renders the page heading and one card per kind", async () => {
    await renderMetadata();
    expect(
      screen.getByRole("heading", { level: 4, name: /metadata/i }),
    ).toBeInTheDocument();
    expect(screen.getByText("SDTM")).toBeInTheDocument();
    expect(screen.getByText("ADaM")).toBeInTheDocument();
  });

  it("renders a disabled Domain Model row in each card", async () => {
    await renderMetadata();
    for (const kind of ["SDTM", "ADaM"] as const) {
      const card = findCardByTitle(kind);
      const domainRow = within(card).getByRole("button", {
        name: /domain model/i,
      });
      expect(domainRow).toHaveAttribute("aria-disabled", "true");
    }
  });

  it("renders an enabled Terminology row in each card", async () => {
    await renderMetadata();
    for (const kind of ["SDTM", "ADaM"] as const) {
      const card = findCardByTitle(kind);
      const termRow = within(card).getByRole("button", { name: /^terminology$/i });
      expect(termRow).not.toHaveAttribute("aria-disabled", "true");
    }
  });

  it("navigates to /terminology/sdtm when the SDTM Terminology row is clicked", async () => {
    const { router } = await renderMetadata();
    const sdtmCard = findCardByTitle("SDTM");
    const sdtmTerm = within(sdtmCard).getByRole("button", {
      name: /^terminology$/i,
    });
    await userEvent.click(sdtmTerm);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/terminology/sdtm"),
    );
  });

  it("navigates to /terminology/adam when the ADaM Terminology row is clicked", async () => {
    const { router } = await renderMetadata();
    const adamCard = findCardByTitle("ADaM");
    const adamTerm = within(adamCard).getByRole("button", {
      name: /^terminology$/i,
    });
    await userEvent.click(adamTerm);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/terminology/adam"),
    );
  });

  it("shows the Coming soon tooltip on the Domain Model rows", async () => {
    await renderMetadata();
    // The disabled button has pointer-events: none; hover its wrapper
    // <span aria-label="Coming soon"> instead.
    const wrappers = screen.getAllByLabelText(/coming soon/i);
    expect(wrappers.length).toBeGreaterThan(0);
    await userEvent.hover(wrappers[0]);
    expect(
      await screen.findByRole("tooltip", { name: /coming soon/i }),
    ).toBeInTheDocument();
  });
});