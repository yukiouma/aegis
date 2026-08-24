import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
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
    getItem(key) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key) {
      data.delete(key);
    },
    setItem(key, value) {
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

describe("MetadataPage", () => {
  it("renders the heading and both block titles", async () => {
    await renderMetadata();
    expect(
      screen.getByRole("heading", { level: 4, name: /metadata/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 6, name: /^SDTM$/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 6, name: /^ADaM$/ }),
    ).toBeInTheDocument();
  });

  it("renders a disabled Domain Model row in each card", async () => {
    await renderMetadata();
    const domainRows = screen.getAllByRole("button", { name: /domain model/i });
    expect(domainRows).toHaveLength(2);
    for (const row of domainRows) {
      expect(row).toBeDisabled();
    }
  });

  it("renders an enabled Terminology row in each card", async () => {
    await renderMetadata();
    const termRows = screen.getAllByRole("button", { name: /^terminology$/i });
    expect(termRows).toHaveLength(2);
    for (const row of termRows) {
      expect(row).not.toBeDisabled();
    }
  });

  it("navigates to /terminology/sdtm when the SDTM Terminology row is clicked", async () => {
    const { router } = await renderMetadata();
    const sdtmCard = screen.getByRole("heading", {
      level: 6,
      name: /^SDTM$/,
    }).parentElement!;
    const sdtmTerm = sdtmCard.querySelector(
      "button:not([disabled])",
    ) as HTMLButtonElement;
    await userEvent.click(sdtmTerm);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/terminology/sdtm"),
    );
  });

  it("navigates to /terminology/adam when the ADaM Terminology row is clicked", async () => {
    const { router } = await renderMetadata();
    const adamCard = screen.getByRole("heading", {
      level: 6,
      name: /^ADaM$/,
    }).parentElement!;
    const adamTerm = adamCard.querySelector(
      "button:not([disabled])",
    ) as HTMLButtonElement;
    await userEvent.click(adamTerm);
    await waitFor(() =>
      expect(router.state.location.pathname).toBe("/terminology/adam"),
    );
  });

  it("shows the Coming soon tooltip on the Domain Model rows", async () => {
    await renderMetadata();
    const firstDomainRow = screen.getAllByRole("button", {
      name: /domain model/i,
    })[0];
    await userEvent.hover(firstDomainRow);
    expect(
      await screen.findByRole("tooltip", { name: /coming soon/i }),
    ).toBeInTheDocument();
  });
});