import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  useAddAssignee,
  useCreateMission,
  useListMissionsByProject,
  useRemoveAssignee,
} from "../../../features/mission";
import { queryKeys } from "../../../shared/query";
import type {
  AssigneeViewResponse,
  CreateMissionInput,
  MissionViewResponse,
} from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const mission: MissionViewResponse = {
  id: 10,
  projectCode: "alpha",
  missionKind: "crf",
  missionCode: "AE",
  assignees: [
    {
      id: 100,
      userCode: "alice",
      role: "dev",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
  ],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const assigneeResp: AssigneeViewResponse = {
  id: 200,
  userCode: "carol",
  role: "qc",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function ListProbe({ projectCode }: { projectCode: string | null }) {
  const q = useListMissionsByProject(projectCode);
  return <span data-testid="count">{q.data?.length ?? "none"}</span>;
}

function AddHarness({ projectCode }: { projectCode: string }) {
  const m = useAddAssignee(projectCode);
  return (
    <button
      onClick={() => {
        m.mutate({
          missionId: 10,
          body: { userCode: "carol", role: "qc" },
        });
      }}
    >
      add
    </button>
  );
}

function RemoveHarness({ projectCode }: { projectCode: string }) {
  const m = useRemoveAssignee(projectCode);
  return (
    <button
      onClick={() => {
        m.mutate({ missionId: 10, assigneeId: 100 });
      }}
    >
      remove
    </button>
  );
}

function CreateHarness({ projectCode }: { projectCode: string }) {
  const m = useCreateMission(projectCode);
  return (
    <button
      onClick={() => {
        const input: CreateMissionInput = {
          projectCode,
          missionKind: "crf",
          missionCode: "VS",
          assignees: [{ userCode: "carol", role: "qc" }],
        };
        m.mutate(input);
      }}
    >
      create
    </button>
  );
}

describe("useListMissionsByProject", () => {
  it("does not fetch when projectCode is null", async () => {
    mockCommands({ list_missions_by_project: () => [mission] });
    renderWithQueryClient(<ListProbe projectCode={null} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("invokes api.listMissionsByProject with { projectCode, kind } and exposes the array", async () => {
    mockCommands({ list_missions_by_project: () => [mission] });
    renderWithQueryClient(<ListProbe projectCode="alpha" />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_missions_by_project", {
        projectCode: "alpha",
        kind: "crf",
      });
      expect(screen.getByTestId("count").textContent).toBe("1");
    });
  });
});

describe("useAddAssignee", () => {
  it("invokes api.addAssignee with { missionId, body }", async () => {
    mockCommands({ add_assignee: () => assigneeResp });
    renderWithQueryClient(<AddHarness projectCode="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "add" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("add_assignee", {
        missionId: 10,
        body: { userCode: "carol", role: "qc" },
      });
    });
  });

  it("invalidates mission.byProject on success", async () => {
    mockCommands({ add_assignee: () => assigneeResp });
    const { client } = renderWithQueryClient(<AddHarness projectCode="alpha" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "add" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha"),
        }),
      );
    });
  });
});

describe("useRemoveAssignee", () => {
  it("invokes api.removeAssignee with { missionId, assigneeId }", async () => {
    mockCommands({ remove_assignee: () => undefined });
    renderWithQueryClient(<RemoveHarness projectCode="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "remove" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("remove_assignee", {
        missionId: 10,
        assigneeId: 100,
      });
    });
  });

  it("invalidates mission.byProject on success", async () => {
    mockCommands({ remove_assignee: () => undefined });
    const { client } = renderWithQueryClient(
      <RemoveHarness projectCode="alpha" />,
    );
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "remove" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha"),
        }),
      );
    });
  });
});

describe("useCreateMission", () => {
  it("invokes api.createMission with the input shape", async () => {
    mockCommands({ create_mission: () => mission });
    renderWithQueryClient(<CreateHarness projectCode="alpha" />);
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("create_mission", {
        projectCode: "alpha",
        missionKind: "crf",
        missionCode: "VS",
        assignees: [{ userCode: "carol", role: "qc" }],
      });
    });
  });

  it("invalidates mission.byProject on success", async () => {
    mockCommands({ create_mission: () => mission });
    const { client } = renderWithQueryClient(
      <CreateHarness projectCode="alpha" />,
    );
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "create" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha"),
        }),
      );
    });
  });
});

describe("mission hooks share the QueryClient cache", () => {
  it("an add-assignee success invalidation reaches the mission.byProject prefix", async () => {
    mockCommands({ add_assignee: () => assigneeResp });
    const { client } = renderWithQueryClient(<AddHarness projectCode="alpha" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "add" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.mission.byProject("alpha"),
        }),
      );
    });
  });
});