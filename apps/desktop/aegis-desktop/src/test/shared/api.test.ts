import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../../shared/api";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("api wrappers", () => {
  it("login -> invoke('login', { code, password })", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.login("alice", "secret");
    expect(mockInvoke).toHaveBeenCalledWith("login", { code: "alice", password: "secret" });
  });

  it("loginDomain -> invoke('login_domain') with no args", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.loginDomain();
    expect(mockInvoke).toHaveBeenCalledWith("login_domain");
  });

  it("isLoggedIn -> invoke('is_logged_in')", async () => {
    mockInvoke.mockResolvedValueOnce(true);
    await api.isLoggedIn();
    expect(mockInvoke).toHaveBeenCalledWith("is_logged_in");
  });

  it("refresh -> invoke('refresh')", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.refresh();
    expect(mockInvoke).toHaveBeenCalledWith("refresh");
  });

  it("logout -> invoke('logout')", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.logout();
    expect(mockInvoke).toHaveBeenCalledWith("logout");
  });

  it("registerUser -> invoke('register_user', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.registerUser({
      userCode: "u",
      userName: "n",
      domainName: "d",
      hostname: "h",
      sid: "s",
      password: "p",
    });
    expect(mockInvoke).toHaveBeenCalledWith("register_user", {
      userCode: "u",
      userName: "n",
      domainName: "d",
      hostname: "h",
      sid: "s",
      password: "p",
    });
  });

  it("updateUserCredential -> invoke('update_user_credential', { userCode, password? })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateUserCredential({ userCode: "u", password: "p" });
    expect(mockInvoke).toHaveBeenCalledWith("update_user_credential", {
      userCode: "u",
      password: "p",
    });
  });

  it("createUser -> invoke('create_user', { code, name, role })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createUser({ code: "u", name: "Alice", role: "admin" });
    expect(mockInvoke).toHaveBeenCalledWith("create_user", {
      code: "u",
      name: "Alice",
      role: "admin",
    });
  });

  it("listUsers -> invoke('list_users')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listUsers();
    expect(mockInvoke).toHaveBeenCalledWith("list_users");
  });

  it("getUserByCode -> invoke('get_user_by_code', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getUserByCode("alice");
    expect(mockInvoke).toHaveBeenCalledWith("get_user_by_code", { code: "alice" });
  });

  it("updateUser -> invoke('update_user', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateUser("alice", { name: "Alicia" });
    expect(mockInvoke).toHaveBeenCalledWith("update_user", {
      code: "alice",
      body: { name: "Alicia" },
    });
  });

  it("createProject -> invoke('create_project', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createProject({ code: "p", description: "" });
    expect(mockInvoke).toHaveBeenCalledWith("create_project", {
      code: "p",
      description: "",
    });
  });

  it("listProjects -> invoke('list_projects')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listProjects();
    expect(mockInvoke).toHaveBeenCalledWith("list_projects");
  });

  it("getProjectByCode -> invoke('get_project_by_code', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getProjectByCode("p");
    expect(mockInvoke).toHaveBeenCalledWith("get_project_by_code", { code: "p" });
  });

  it("updateProject -> invoke('update_project', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateProject("p", { active: false });
    expect(mockInvoke).toHaveBeenCalledWith("update_project", {
      code: "p",
      body: { active: false },
    });
  });

  it("healthz -> invoke('healthz')", async () => {
    mockInvoke.mockResolvedValueOnce("ok");
    await api.healthz();
    expect(mockInvoke).toHaveBeenCalledWith("healthz");
  });
});
