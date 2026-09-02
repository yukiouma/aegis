import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, waitFor } from "@testing-library/react";
import type { ReactElement } from "react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useIsProjectLeader } from "../../../features/mission";
import type { ProjectView, UserView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";
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

  // Regression guard for the assign-mission-drawer table shake.
  // `useIsProjectLeader` shares `queryKeys.project.byCode(code)` with
  // the drawer's `useProject(code, { enabled: open })`. Opening the
  // drawer enables the second observer, which (with `staleTime: 0`)
  // triggers a refetch. Before the fix, the hook returned `null`
  // whenever `projectQuery.isFetching` was true — so the refetch
  // flipped `isLeader` from `true` to `null`, briefly hiding the
  // per-row assign-mission icon and shaking the table. The hook now
  // returns the cached `true` during refetches.
  it("preserves the cached leader result during a refetch (does not flash to null)", async () => {
    // Make the refetch deliberately slow so React has time to
    // render the intermediate `isFetching: true` state. Without
    // this delay the test's network round-trip completes in the
    // same microtask as the invalidate and React never observes
    // the buggy intermediate value.
    let resolveRefetch!: (v: ProjectView) => void;
    const pendingRefetch = new Promise<ProjectView>((r) => {
      resolveRefetch = r;
    });
    let projectCalls = 0;
    mockCommands({
      current_user: () => alice,
      get_project_by_code: () => {
        projectCalls += 1;
        if (projectCalls === 1) return projectAliceLeader;
        return pendingRefetch;
      },
    });

    // Capture every value the hook produces across renders.
    const history: (boolean | null)[] = [];
    const { client } = renderWithQueryClient(
      <Capture projectCode="alpha" sink={(v) => history.push(v)} />,
    );

    // Wait until the initial fetch resolves to true.
    await waitFor(() => expect(history[history.length - 1]).toBe(true));

    // Trigger a refetch via invalidation — the same sequence the
    // assign-mission drawer produces when its `useProject` enables.
    void client.invalidateQueries({
      queryKey: queryKeys.project.byCode("alpha"),
    });

    // Wait until the second fetch is in flight (mock invoked twice,
    // second call hanging on `pendingRefetch`). At this moment
    // `projectQuery.isFetching` is true; if the bug were present
    // the hook would render `null` here.
    const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;
    await waitFor(() => expect(mockInvoke.mock.calls.length).toBeGreaterThanOrEqual(2));

    // Sample the most recent value while the refetch is still pending.
    // With the fix: `true` (cached). Without the fix: `null`.
    expect(history[history.length - 1]).toBe(true);

    // Resolve the refetch so React Query can settle.
    resolveRefetch(projectAliceLeader);

    // Confirm the entire post-`true` history stayed `true`.
    const firstTrueIdx = history.indexOf(true);
    expect(firstTrueIdx).toBeGreaterThanOrEqual(0);
    const afterFirstTrue = history.slice(firstTrueIdx);
    expect(afterFirstTrue.every((v) => v === true)).toBe(true);
  });
});