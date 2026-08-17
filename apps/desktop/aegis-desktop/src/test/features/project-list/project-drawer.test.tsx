import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { ProjectDrawer } from "../../../features/project-list/components/ProjectDrawer";
import type {
  ProjectView,
  UpdateProjectBody,
  UserView,
} from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderInRouter } from "../../../test/helpers/file-route-utils";

const userFixture: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const userFixture2: UserView = {
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha description",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [],
    workers: [],
  },
  tags: [],
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

function renderDrawer(
  mode: "closed" | "create" | "edit",
  code: string | null = null,
) {
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ProjectDrawer mode={mode} code={code} onClose={vi.fn()} />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("ProjectDrawer — closed", () => {
  it("does not render any form fields when mode is 'closed'", async () => {
    await renderDrawer("closed");
    expect(screen.queryByLabelText(/\bcode\b/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/\bdescription\b/i)).not.toBeInTheDocument();
  });
});

describe("ProjectDrawer — create mode", () => {
  it("shows 'Create project' title and an enabled code field", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    expect(
      screen.getByRole("heading", { name: /create project/i }),
    ).toBeInTheDocument();
    const codeField = screen.getByLabelText(/\bcode\b/i);
    expect(codeField).toBeInTheDocument();
    expect(codeField).not.toBeDisabled();
  });

  it("does not show the active switch in create mode", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
  });

  it("disables Submit until code and description are set", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
    });
    await renderDrawer("create");
    const submit = await screen.findByRole("button", { name: /^create$/i });
    expect(submit).toBeDisabled();
  });

  it("calls api.createProject with the assembled shape on Submit", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
      create_project: () => projectFixture,
    });
    await renderDrawer("create");

    await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
    await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");

    const memberLeadersInput = screen.getByLabelText(/^members\s*—\s*leaders$/i);
    await userEvent.click(memberLeadersInput);
    await userEvent.click(screen.getByRole("option", { name: /alice/i }));

    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "create_project",
        expect.objectContaining({
          code: "newproj",
          description: "New desc",
          members: expect.objectContaining({
            leaders: expect.arrayContaining(["alice"]),
          }),
        }),
      );
    });
  });
});

describe("ProjectDrawer — edit mode", () => {
  it("fetches the project via get_project_by_code and pre-fills the form", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
      get_project_by_code: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_by_code", {
        code: "alpha",
      });
    });

    const codeField = await screen.findByLabelText(/\bcode\b/i);
    expect(codeField).toBeDisabled();
    expect(codeField).toHaveValue("alpha");
    expect(screen.getByLabelText(/\bdescription\b/i)).toHaveValue(
      "Alpha description",
    );
    expect(screen.getByRole("switch")).toBeInTheDocument();
  });

  it("calls api.updateProject with { code, body } (no code in body) on Submit", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
      get_project_by_code: () => projectFixture,
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");

    const descriptionField = await screen.findByLabelText(/\bdescription\b/i);
    await waitFor(() =>
      expect(descriptionField).toHaveValue("Alpha description"),
    );

    fireEvent.change(descriptionField, { target: { value: "Edited" } });

    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));

    const expectedBody: UpdateProjectBody = expect.objectContaining({
      description: "Edited",
      active: true,
    });
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_project", {
        code: "alpha",
        body: expectedBody,
      });
    });
  });
});

describe("ProjectDrawer — mutation error", () => {
  it("shows an Alert with the error message when create_project fails", async () => {
    mockCommands({
      list_users: () => [userFixture, userFixture2],
      create_project: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    await renderDrawer("create");

    await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
    await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "New desc");

    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /server: boom/i,
    );
  });
});

describe("ProjectDrawer — tags", () => {
  it("renders the TagEditor in create mode with zero rows by default", async () => {
    mockCommands({ list_users: () => [userFixture] });
    await renderDrawer("create");
    expect(screen.getByRole("button", { name: /add tag/i })).toBeInTheDocument();
    // No rows means no key/value labels yet.
    expect(screen.queryAllByLabelText(/tag key/i)).toHaveLength(0);
  });

  it("seeds the editor with the project's tags in edit mode and tagsTouched stays false", async () => {
    mockCommands({
      list_users: () => [userFixture],
      get_project_by_code: () => ({
        ...projectFixture,
        tags: [{ key: "Product", value: "DEMO-001" }],
      }),
    });
    await renderDrawer("edit", "alpha");
    await waitFor(() =>
      expect(screen.getByLabelText(/tag value/i)).toHaveValue("DEMO-001"),
    );
  });

  it("create-mode submit includes the assembled tags array on the body", async () => {
    mockCommands({
      list_users: () => [userFixture],
      create_project: () => projectFixture,
    });
    await renderDrawer("create");
    await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
    await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "d");
    await userEvent.click(screen.getByRole("button", { name: /add tag/i }));
    const keyInput = screen.getByLabelText(/tag key/i);
    const valueInput = screen.getByLabelText(/tag value/i);
    await userEvent.type(keyInput, "Product");
    await userEvent.type(valueInput, "DEMO-007");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_project",
        expect.objectContaining({
          tags: [{ key: "Product", value: "DEMO-007" }],
        }),
      ),
    );
  });

  it("edit-mode submit omits tags from the body when user did NOT touch the editor", async () => {
    mockCommands({
      list_users: () => [userFixture],
      get_project_by_code: () => ({
        ...projectFixture,
        tags: [{ key: "Product", value: "DEMO-001" }],
      }),
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");
    const descriptionField = await screen.findByLabelText(/\bdescription\b/i);
    await waitFor(() =>
      expect(descriptionField).toHaveValue("Alpha description"),
    );
    // Edit the description (which does NOT touch the tag editor) — the
    // body should omit `tags` so the server leaves them alone.
    fireEvent.change(descriptionField, { target: { value: "Edited" } });
    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));
    await waitFor(() => {
      const call = (invoke as unknown as ReturnType<typeof vi.fn>).mock.calls.find(
        ([cmd]) => cmd === "update_project",
      );
      expect(call).toBeDefined();
      const body = call![1].body as UpdateProjectBody;
      expect(body).not.toHaveProperty("tags");
    });
  });

  it("edit-mode submit sends the new tags array when user edited the editor", async () => {
    mockCommands({
      list_users: () => [userFixture],
      get_project_by_code: () => ({
        ...projectFixture,
        tags: [{ key: "Product", value: "DEMO-001" }],
      }),
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");
    await screen.findByLabelText(/\bdescription\b/i);
    const valueInput = await screen.findByLabelText(/tag value/i);
    // Wait for the seed to render before driving the change.
    await waitFor(() => expect(valueInput).toHaveValue("DEMO-001"));
    fireEvent.change(valueInput, { target: { value: "DEMO-002" } });
    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));
    await waitFor(() => {
      const call = (invoke as unknown as ReturnType<typeof vi.fn>).mock.calls.find(
        ([cmd]) => cmd === "update_project",
      );
      expect(call).toBeDefined();
      const body = call![1].body as UpdateProjectBody;
      expect(body.tags).toEqual([{ key: "Product", value: "DEMO-002" }]);
    });
  });
});
