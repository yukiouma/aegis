import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useIsProjectLeader } from "../../../features/mission";
import type { ProjectView, UserView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const alice: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const carol: UserView = {
  id: 2,
  code: "carol",
  name: "Carol",
  role: "general",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectAliceLeader: ProjectView = {
  id: 7,
  code: "alpha",
  description: "",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "carol", name: "Carol" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  tags: [],
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const projectUnblindLeader: ProjectView = {
  ...projectAliceLeader,
  unblindMembers: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [],
  },
};

const projectNoLeader: ProjectView = {
  ...projectAliceLeader,
  members: { leaders: [], workers: [{ code: "alice", name: "Alice" }] },
  unblindMembers: { leaders: [], workers: [] },
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

interface CapturedValue {
  v: boolean | null;
}

function Capture({
  projectCode,
  sink,
}: {
  projectCode: string | null;
  sink: (v: boolean | null) => void;
}): ReactElement {
  const v = useIsProjectLeader(projectCode);
  sink(v);
  return <span data-testid="result">{String(v)}</span>;
}

async function renderAndGet(
  projectCode: string | null,
  project: ProjectView | undefined,
  current: UserView | undefined,
): Promise<boolean | null> {
  const handlers: Record<string, (args?: Record<string, unknown>) => unknown> =
    {};
  if (project) {
    handlers.get_project_by_code = () => project;
  }
  if (current) {
    handlers.current_user = () => current;
  }
  mockCommands(handlers);
  const captured: CapturedValue = { v: null };
  renderWithQueryClient(
    <Capture projectCode={projectCode} sink={(v) => (captured.v = v)} />,
  );
  // For the disabled gate (projectCode is null) the project query
  // never fires and the hook stays null; we still need a tick so the
  // initial render has settled.
  if (projectCode == null || projectCode === "") {
    await new Promise((r) => setTimeout(r, 0));
    return captured.v;
  }
  // Otherwise wait for both queries to have fired (if enabled), then
  // give React Query one more microtask to settle so the memo re-runs.
  if (project) {
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_by_code", {
        code: projectCode,
      });
    });
  }
  if (current) {
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("current_user");
    });
  }
  await waitFor(() => {
    expect(captured.v).not.toBe(null);
  });
  return captured.v;
}

describe("useIsProjectLeader", () => {
  it("returns null when projectCode is null (gate disabled)", async () => {
    const v = await renderAndGet(null, undefined, undefined);
    // current_user still fires once (always-enabled hook), but
    // get_project_by_code is gated — that's the only assertion that
    // matters for the leader gate.
    expect(v).toBeNull();
    expect(invoke).not.toHaveBeenCalledWith(
      "get_project_by_code",
      expect.anything(),
    );
  });

  it("returns true when the current user is in members.leaders", async () => {
    const v = await renderAndGet("alpha", projectAliceLeader, alice);
    expect(v).toBe(true);
  });

  it("returns false when the current user is only a worker", async () => {
    const v = await renderAndGet("alpha", projectAliceLeader, carol);
    expect(v).toBe(false);
  });

  it("returns true when the current user is in unblindMembers.leaders", async () => {
    const v = await renderAndGet("alpha", projectUnblindLeader, alice);
    expect(v).toBe(true);
  });

  it("returns false when no leader membership exists", async () => {
    const v = await renderAndGet("alpha", projectNoLeader, alice);
    expect(v).toBe(false);
  });
});