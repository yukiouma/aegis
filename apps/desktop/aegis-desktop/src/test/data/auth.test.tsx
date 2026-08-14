import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useLogin, useLoginDomain } from "../../data/auth";
import { queryKeys } from "../../data/queryKeys";
import { mockCommands } from "../tauri-mock";
import { renderWithQueryClient } from "../render-with-query-client";

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function LoginHarness({
  variant,
}: {
  variant: "account" | "domain";
}) {
  const login = useLogin();
  const loginDomain = useLoginDomain();
  return (
    <>
      <button
        onClick={() => {
          if (variant === "domain") {
            loginDomain.mutate();
          } else {
            login.mutate({ code: "alice", password: "secret" });
          }
        }}
      >
        submit
      </button>
      <span data-testid="pending">
        {login.isPending || loginDomain.isPending ? "yes" : "no"}
      </span>
      <span data-testid="error-kind">
        {login.error?.kind ?? loginDomain.error?.kind ?? "none"}
      </span>
    </>
  );
}

function LoginStatusProbe({
  client,
}: {
  client: ReturnType<typeof renderWithQueryClient>["client"];
}) {
  // Tiny consumer of the login-status query key so we can assert its
  // invalidation state via the QueryClient directly.
  useEffect(() => {
    void client;
  }, [client]);
  return null;
}

describe("useLogin", () => {
  it("invokes api.login with code and password on mutate()", async () => {
    mockCommands({ login: () => undefined });
    renderWithQueryClient(<LoginHarness variant="account" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("login", {
        code: "alice",
        password: "secret",
      });
    });
  });

  it("invalidates queryKeys.auth.loginStatus on success", async () => {
    mockCommands({ login: () => undefined });
    const { client } = renderWithQueryClient(<LoginHarness variant="account" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.auth.loginStatus(),
        }),
      );
    });
  });

  it("exposes the thrown ApiError through mutation.error", async () => {
    mockCommands({
      login: () => {
        throw {
          kind: "http",
          status: 401,
          code: "invalid_credentials",
          message: "bad",
        };
      },
    });
    renderWithQueryClient(<LoginHarness variant="account" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });

  it("toggles isPending across the call lifecycle", async () => {
    let resolve!: () => void;
    mockCommands({
      login: () => new Promise<void>((r) => {
        resolve = r;
      }),
    });
    renderWithQueryClient(<LoginHarness variant="account" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(screen.getByTestId("pending").textContent).toBe("yes");
    });
    resolve();
    await waitFor(() => {
      expect(screen.getByTestId("pending").textContent).toBe("no");
    });
  });
});

describe("useLoginDomain", () => {
  it("invokes api.loginDomain with no args on mutate()", async () => {
    mockCommands({ login_domain: () => undefined });
    renderWithQueryClient(<LoginHarness variant="domain" />);
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("login_domain");
    });
  });

  it("invalidates queryKeys.auth.loginStatus on success", async () => {
    mockCommands({ login_domain: () => undefined });
    const { client } = renderWithQueryClient(<LoginHarness variant="domain" />);
    const spy = vi.spyOn(client, "invalidateQueries");
    await userEvent.click(screen.getByRole("button", { name: "submit" }));
    await waitFor(() => {
      expect(spy).toHaveBeenCalledWith(
        expect.objectContaining({
          queryKey: queryKeys.auth.loginStatus(),
        }),
      );
    });
  });
});

// `LoginStatusProbe` is a deliberate no-op consumer of `client` so the
// effect dependency on `client` doesn't trip `react-hooks/exhaustive-deps`.
// It is intentionally not rendered in any test — kept here only to
// document the cache-key assertion strategy.
void LoginStatusProbe;
// `render` import is used transitively by `renderWithQueryClient`'s
// return type; importing it here keeps the helper's signature obvious.
void render;