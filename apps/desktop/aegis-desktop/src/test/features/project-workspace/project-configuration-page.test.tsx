import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { renderWithFullRouter } from "../../../test/helpers/file-route-utils";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import type { ProjectView, UserView } from "../../../shared/api";
import { makeProject } from "../../helpers/project-fixture";

const leader: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const other: UserView = {
  id: 2,
  code: "carol",
  name: "Carol",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectFixture: ProjectView = makeProject({
  code: "alpha",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "bob", name: "Bob" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  configurations: {
    language: "en",
    tags: [{ key: "Product", value: "DEMO-001" }],
  },
});

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  vi.stubGlobal("localStorage", {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    get length() {
      return 0;
    },
  });
});
afterEach(() => cleanup());

function renderPage(initialEntry = "/project/alpha/configuration") {
  return renderWithFullRouter({
    initialEntries: [initialEntry],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

describe("ProjectConfigurationPage — leader view", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
    });
  });

  it("renders the heading", async () => {
    await renderPage();
    expect(
      await screen.findByRole("heading", { name: /configuration/i }),
    ).toBeInTheDocument();
  });

  it("renders the three-section sidebar with General selected", async () => {
    await renderPage();
    const general = await screen.findByTestId("config-section-general");
    const members = await screen.findByTestId("config-section-members");
    const filepath = await screen.findByTestId("config-section-filepath");
    // MUI ListItemButton uses the `Mui-selected` class for the active
    // item rather than `aria-selected`.
    expect(general.className).toMatch(/Mui-selected/);
    expect(members.className).not.toMatch(/Mui-selected/);
    expect(filepath.className).not.toMatch(/Mui-selected/);
  });

  it("does NOT render the read-only banner for a project leader", async () => {
    await renderPage();
    await screen.findByTestId("config-section-general");
    expect(screen.queryByTestId("config-readonly")).not.toBeInTheDocument();
  });

  it("Save button starts disabled and enables after a tag change", async () => {
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    // Type into the existing tag's value field to flip `tagsTouched`.
    // TagEditor uses TextField, which userEvent.click+type handles cleanly.
    const valueInput = await screen.findByLabelText(/tag value/i);
    await userEvent.clear(valueInput);
    await userEvent.type(valueInput, "DEMO-002");

    await waitFor(() => expect(save).not.toBeDisabled());
  });

  it("clicking Save fires update_project with configurations { language, tags }", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      update_project: () => projectFixture,
    });
    await renderPage();

    // Edit the existing tag's value to flip `tagsTouched`. The whole
    // configurations object is then sent on save.
    const valueInput = await screen.findByLabelText(/tag value/i);
    await userEvent.clear(valueInput);
    await userEvent.type(valueInput, "DEMO-002");

    const save = await screen.findByTestId("config-general-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              language: "en",
              tags: expect.arrayContaining([
                { key: "Product", value: "DEMO-002" },
              ]),
            }),
          }),
        }),
      ),
    );
  });

  it("clicking the Members sidebar item shows the Members section", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-members"));
    expect(await screen.findByTestId("config-leaders")).toBeInTheDocument();
    expect(await screen.findByTestId("config-workers")).toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — non-leader view", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => other,
      get_project_by_code: () => projectFixture,
    });
  });

  it("renders the read-only banner and hides Save buttons", async () => {
    await renderPage();
    expect(await screen.findByTestId("config-readonly")).toBeInTheDocument();
    expect(
      screen.queryByTestId("config-general-save"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("config-members-save"),
    ).not.toBeInTheDocument();
  });

  it("language Select is disabled for non-leaders", async () => {
    await renderPage();
    // The Select wraps in a FormControl; when disabled, the form
    // control root has the `Mui-disabled` class.
    const select = await screen.findByLabelText(/preferred language/i);
    await waitFor(() => {
      const fc = select.closest(".MuiFormControl-root");
      expect(fc).toBeTruthy();
      // The disabled class lands on the inner Select wrapper rather
      // than the form control root; check both possibilities.
      const root = select.closest(".MuiInputBase-root");
      expect(
        fc!.classList.contains("Mui-disabled") ||
          root!.classList.contains("Mui-disabled"),
      ).toBe(true);
    });
  });
});

describe("ProjectConfigurationPage — filepath placeholder", () => {
  beforeEach(() => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
    });
  });

  it("clicking Filepath shows the placeholder", async () => {
    await renderPage();
    await userEvent.click(await screen.findByTestId("config-section-filepath"));
    expect(
      await screen.findByTestId("config-filepath-placeholder"),
    ).toBeInTheDocument();
  });
});

describe("ProjectConfigurationPage — error path", () => {
  it("renders an Alert when get_project_by_code fails", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => {
        throw {
          kind: "http",
          status: 500,
          code: "server",
          message: "boom",
        };
      },
    });
    await renderPage();
    expect(await screen.findByRole("alert")).toHaveTextContent(/server/i);
  });
});