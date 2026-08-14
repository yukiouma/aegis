import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import {
  useCurrentUser,
  useDomainUserInfo,
  useLogout,
  useRegisterUser,
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

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
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
});