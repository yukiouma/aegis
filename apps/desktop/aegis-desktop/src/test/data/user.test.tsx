import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockGetAll = vi.fn();
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getAllWebviewWindows: (...args: unknown[]) => mockGetAll(...args),
}));

import {
  useCurrentUser,
  useDomainUserInfo,
  useListUsers,
  useLogout,
  useRegisterUser,
  useUpdateUser,
} from "../../data/user";
import { queryKeys } from "../../data/queryKeys";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

const userView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin" as const,
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const identity = {
  domain: "EXAMPLE",
  hostMachine: "host01",
  sid: "S-1-...",
  userid: "alice",
};

const usersList = [
  userView,
  {
    id: 2,
    code: "bob",
    name: "Bob",
    role: "general" as const,
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
];

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  mockGetAll.mockReset();
  // Default to no other windows so existing tests don't break.
  // Individual tests override this when they need a populated list.
  mockGetAll.mockResolvedValue([]);
});
afterEach(() => {
  cleanup();
});

function CurrentUserProbe() {
  const q = useCurrentUser();
  return <span data-testid="data">{q.data?.name ?? "none"}</span>;
}

function DomainInfoProbe({ enabled }: { enabled?: boolean }) {
  const q = useDomainUserInfo({ enabled });
  return (
    <>
      <button onClick={() => void q.refetch()}>refetch</button>
      <span data-testid="userid">{q.data?.userid ?? "none"}</span>
    </>
  );
}

function RegisterHarness() {
  const m = useRegisterUser();
  return (
    <>
      <button
        onClick={() => {
          m.mutate({
            userCode: "alice",
            userName: "Alice",
            domainName: "EXAMPLE",
            hostname: "host01",
            sid: "S-1-...",
            password: "pw",
          });
        }}
      >
        register
      </button>
      <span data-testid="pending">{m.isPending ? "yes" : "no"}</span>
    </>
  );
}

function LogoutHarness() {
  const m = useLogout();
  return (
    <button
      onClick={() => {
        m.mutate();
      }}
    >
      logout
    </button>
  );
}

describe("useCurrentUser", () => {
  it("fetches api.current_user on mount", async () => {
    mockCommands({ current_user: () => userView });
    renderWithQueryClient(<CurrentUserProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("current_user");
      expect(screen.getByTestId("data").textContent).toBe("Alice");
    });
  });
});

describe("useDomainUserInfo", () => {
  it("does not fetch on mount when enabled defaults to false", async () => {
    mockCommands({ get_domain_user_info: () => identity });
    renderWithQueryClient(<DomainInfoProbe />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });

  it("returns the Identity payload on manual refetch()", async () => {
    mockCommands({ get_domain_user_info: () => identity });
    renderWithQueryClient(<DomainInfoProbe />);
    await userEvent.click(screen.getByRole("button", { name: "refetch" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_domain_user_info");
      expect(screen.getByTestId("userid").textContent).toBe("alice");
    });
  });
});

describe("useRegisterUser", () => {
  it("invokes api.register_user with the input shape", async () => {
    mockCommands({ register_user: () => ({}) });
    renderWithQueryClient(<RegisterHarness />);
    await userEvent.click(screen.getByRole("button", { name: "register" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("register_user", {
        userCode: "alice",
        userName: "Alice",
        domainName: "EXAMPLE",
        hostname: "host01",
        sid: "S-1-...",
        password: "pw",
      });
    });
  });

  it("does not invalidate any query on success", async () => {
    mockCommands({ register_user: () => ({}) });
    const { client } = renderWithQueryClient(<RegisterHarness />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "register" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("register_user", expect.anything());
    });
    expect(spy).not.toHaveBeenCalled();
  });
});

describe("useLogout", () => {
  it("invokes api.logout with no args", async () => {
    mockCommands({ logout: () => undefined });
    renderWithQueryClient(<LogoutHarness />);
    await userEvent.click(screen.getByRole("button", { name: "logout" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("logout");
    });
  });

  it("clears the entire cache on success", async () => {
    mockCommands({ logout: () => undefined });
    const { client } = renderWithQueryClient(<LogoutHarness />);
    // Seed a stale cache entry to prove it gets wiped.
    client.setQueryData(queryKeys.user.current(), userView);
    expect(client.getQueryData(queryKeys.user.current())).toEqual(userView);

    const clearSpy = vi.spyOn(client, "clear");
    await userEvent.click(screen.getByRole("button", { name: "logout" }));
    await waitFor(() => {
      expect(clearSpy).toHaveBeenCalled();
      expect(client.getQueryData(queryKeys.user.current())).toBeUndefined();
    });
  });

  it("closes every project:* window on success and skips the main window", async () => {
    const mainClose = vi.fn();
    const project1Close = vi.fn();
    const project2Close = vi.fn();
    mockGetAll.mockResolvedValue([
      { label: "main", close: mainClose },
      { label: "project:DEMO-001", close: project1Close },
      { label: "project:DEMO-002", close: project2Close },
    ]);
    mockCommands({ logout: () => undefined });
    const { client } = renderWithQueryClient(<LogoutHarness />);
    const clearSpy = vi.spyOn(client, "clear");

    await userEvent.click(screen.getByRole("button", { name: "logout" }));

    await waitFor(() => {
      expect(project1Close).toHaveBeenCalledTimes(1);
      expect(project2Close).toHaveBeenCalledTimes(1);
      expect(mainClose).not.toHaveBeenCalled();
      expect(clearSpy).toHaveBeenCalled();
    });
  });

  it("does not call any window.close when only the main window exists", async () => {
    const mainClose = vi.fn();
    mockGetAll.mockResolvedValue([{ label: "main", close: mainClose }]);
    mockCommands({ logout: () => undefined });
    const { client } = renderWithQueryClient(<LogoutHarness />);
    const clearSpy = vi.spyOn(client, "clear");

    await userEvent.click(screen.getByRole("button", { name: "logout" }));

    await waitFor(() => expect(clearSpy).toHaveBeenCalled());
    expect(mainClose).not.toHaveBeenCalled();
  });

  it("closes project windows BEFORE clearing the cache", async () => {
    // Deferred close promise — lets the test observe the ordering.
    let closeProject!: () => void;
    const project1Close = vi.fn(
      () => new Promise<void>((resolve) => { closeProject = resolve; }),
    );
    mockGetAll.mockResolvedValue([
      { label: "main", close: vi.fn() },
      { label: "project:DEMO-001", close: project1Close },
    ]);
    mockCommands({ logout: () => undefined });
    const { client } = renderWithQueryClient(<LogoutHarness />);
    const clearSpy = vi.spyOn(client, "clear");

    // Fire the click inside act() so React Query's internal scheduling
    // fully flushes through the synchronous handler entry point.
    // The close() promise stays pending, so we can observe the
    // intermediate state before resolving it.
    let clickPromise!: Promise<void>;
    await act(async () => {
      clickPromise = userEvent.click(
        screen.getByRole("button", { name: "logout" }),
      );
    });

    await waitFor(() => expect(project1Close).toHaveBeenCalledTimes(1));
    expect(clearSpy).not.toHaveBeenCalled();

    closeProject();
    await clickPromise;

    await waitFor(() => expect(clearSpy).toHaveBeenCalled());
  });
});

function ListUsersProbe({ enabled }: { enabled?: boolean }) {
  const q = useListUsers({ enabled });
  return (
    <span data-testid="count">{q.data?.length ?? "none"}</span>
  );
}

describe("useListUsers", () => {
  it("invokes api.listUsers on mount when enabled defaults to true", async () => {
    mockCommands({ list_users: () => usersList });
    renderWithQueryClient(<ListUsersProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_users");
      expect(screen.getByTestId("count").textContent).toBe("2");
    });
  });

  it("does not fetch on mount when enabled is false", async () => {
    mockCommands({ list_users: () => usersList });
    renderWithQueryClient(<ListUsersProbe enabled={false} />);
    await new Promise((r) => setTimeout(r, 0));
    expect(invoke).not.toHaveBeenCalled();
  });
});

function UpdateUserHarness() {
  const m = useUpdateUser();
  return (
    <>
      <button
        onClick={() => {
          m.mutate({ code: "bob", body: { active: false } });
        }}
      >
        toggle
      </button>
      <span data-testid="pending">{m.isPending ? "yes" : "no"}</span>
    </>
  );
}

describe("useUpdateUser", () => {
  it("invokes api.update_user with { code, body }", async () => {
    mockCommands({ update_user: () => userView });
    renderWithQueryClient(<UpdateUserHarness />);
    await userEvent.click(screen.getByRole("button", { name: "toggle" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { active: false },
      });
    });
  });

  it("invalidates user.list and user.current on success", async () => {
    mockCommands({ update_user: () => userView });
    const { client } = renderWithQueryClient(<UpdateUserHarness />);
    client.setQueryData(queryKeys.user.list(), usersList);
    client.setQueryData(queryKeys.user.current(), userView);

    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "toggle" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith({ queryKey: queryKeys.user.list() });
      expect(spy).toHaveBeenCalledWith({ queryKey: queryKeys.user.current() });
    });
  });

  it("does not invalidate any query on error", async () => {
    mockCommands({
      update_user: () =>
        Promise.reject({
          kind: "http",
          status: 403,
          code: "forbidden",
          message: "nope",
        }),
    });
    const { client } = renderWithQueryClient(<UpdateUserHarness />);
    client.setQueryData(queryKeys.user.list(), usersList);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "toggle" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", expect.anything());
    });
    expect(spy).not.toHaveBeenCalled();
  });
});

function UpdateUserRoleHarness() {
  const m = useUpdateUser();
  return (
    <button
      onClick={() => {
        m.mutate({ code: "bob", body: { role: "admin" } });
      }}
    >
      promote
    </button>
  );
}

describe("useUpdateUser — role body", () => {
  it("invokes api.update_user with body: { role }", async () => {
    mockCommands({ update_user: () => userView });
    renderWithQueryClient(<UpdateUserRoleHarness />);
    await userEvent.click(screen.getByRole("button", { name: "promote" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("update_user", {
        code: "bob",
        body: { role: "admin" },
      });
    });
  });
});