import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  useCreateProject,
  useListProjects,
  useProject,
  useUpdateProject,
} from "../../../features/project-list";
import { queryKeys } from "../../../shared/query";
import type { ProjectView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import {
  makeTestQueryClient,
  renderWithQueryClient,
} from "../../../test/helpers/render-with-query-client";

const projectFixture: ProjectView = {
  id: 1,
  code: "alpha",
  description: "Alpha project description",
  product: {
    id: 10,
    code: "prod-a",
    name: "Product A",
    description: "Product A description",
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "carol", name: "Carol" }],
  },
  unblindMembers: {
    leaders: [{ code: "bob", name: "Bob" }],
    workers: [],
  },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function ListProbe() {
  const q = useListProjects();
  return <span data-testid="count">{q.data?.length ?? "none"}</span>;
}

function SingleProbe({ code }: { code: string | null }) {
  const q = useProject(code);
  return (
    <>
      <button onClick={() => void q.refetch()}>refetch</button>
      <span data-testid="project-code">{q.data?.code ?? "none"}</span>
    </>
  );
}

function CreateHarness() {
  const m = useCreateProject();
  return (
    <button
      onClick={() => {
        m.mutate({
          code: "newproj",
          description: "New",
          productId: 10,
          members: { leaders: [], workers: [] },
          unblindMembers: { leaders: [], workers: [] },
        });
      }}
    >
      create
    </button>
  );
}

function UpdateHarness({ code }: { code: string }) {
  const m = useUpdateProject();
  return (
    <button
      onClick={() => {
        m.mutate({ code, body: { description: "Edited" } });
      }}
    >
      update
    </button>
  );
}

describe("useListProjects", () => {
  it("invokes api.listProjects on mount and exposes the array", async () => {
    mockCommands({ list_projects: () => [projectFixture] });
    renderWithQueryClient(<ListProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_projects");
      expect(screen.getByTestId("count").textContent).toBe("1");
    });
  });

  it("refetches when remounted against the same client (reopen /projects)", async () => {
    // The page navigates away (unmount) and back (remount) with the
    // shared QueryClient. With `staleTime: 0` the cached data is
    // immediately stale, so re-mounting triggers a fresh fetch.
    mockCommands({ list_projects: () => [projectFixture] });
    const client = makeTestQueryClient();
    const first = renderWithQueryClient(<ListProbe />, { client });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    first.unmount();
    renderWithQueryClient(<ListProbe />, { client });
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });

  it("propagates ApiError into query.error", async () => {
    mockCommands({
      list_projects: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    function ErrorProbe() {
      const q = useListProjects();
      return (
        <span data-testid="error-kind">
          {q.error ? (q.error as { kind: string }).kind : "none"}
        </span>
      );
    }
    renderWithQueryClient(<ErrorProbe />);
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });
});

describe("useProject", () => {
  it("does not fetch on mount when code is null (disabled)", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code={null} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("does not fetch on mount even when code is set (manual-trigger, enabled:false)", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code="alpha" />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("refetch() invokes api.getProjectByCode with the code", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_by_code", {
        code: "alpha",
      });
      expect(screen.getByTestId("project-code").textContent).toBe("alpha");
    });
  });

  it("two consecutive refetch() calls both hit the server (staleTime: 0)", async () => {
    mockCommands({ get_project_by_code: () => projectFixture });
    renderWithQueryClient(<SingleProbe code="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(1));
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  });
});

describe("useCreateProject", () => {
  it("invokes api.createProject with the input shape", async () => {
    mockCommands({ create_project: () => projectFixture });
    renderWithQueryClient(<CreateHarness />);
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_project", {
        code: "newproj",
        description: "New",
        productId: 10,
        members: { leaders: [], workers: [] },
        unblindMembers: { leaders: [], workers: [] },
      });
    });
  });

  it("invalidates queryKeys.project.all() on success", async () => {
    mockCommands({ create_project: () => projectFixture });
    const { client } = renderWithQueryClient(<CreateHarness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: queryKeys.project.all() }),
      );
    });
  });

  it("does not clear the entire cache (unlike logout)", async () => {
    mockCommands({ create_project: () => projectFixture });
    const rendered = renderWithQueryClient(<CreateHarness />);
    rendered.client.setQueryData(queryKeys.project.all(), [projectFixture]);
    const clearSpy = vi.spyOn(rendered.client, "clear");
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("create_project", expect.anything()),
    );
    expect(clearSpy).not.toHaveBeenCalled();
  });
});

describe("useUpdateProject", () => {
  it("invokes api.updateProject with { code, body }", async () => {
    mockCommands({ update_project: () => projectFixture });
    renderWithQueryClient(<UpdateHarness code="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "update" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_project", {
        code: "alpha",
        body: { description: "Edited" },
      });
    });
  });

  it("invalidates queryKeys.project.all() AND project.byCode(code) on success", async () => {
    mockCommands({ update_project: () => projectFixture });
    const { client } = renderWithQueryClient(<UpdateHarness code="alpha" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "update" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({ queryKey: queryKeys.project.all() }),
      );
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.project.byCode("alpha"),
        }),
      );
    });
  });
});