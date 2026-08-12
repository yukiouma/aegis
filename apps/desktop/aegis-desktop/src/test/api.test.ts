import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../api";

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

  it("loginDomain -> invoke('loginDomain', { code })", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.loginDomain("alice");
    expect(mockInvoke).toHaveBeenCalledWith("loginDomain", { code: "alice" });
  });

  it("isLoggedIn -> invoke('isLoggedIn')", async () => {
    mockInvoke.mockResolvedValueOnce(true);
    await api.isLoggedIn();
    expect(mockInvoke).toHaveBeenCalledWith("isLoggedIn");
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

  it("registerUser -> invoke('registerUser', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.registerUser({
      userCode: "u",
      userName: "n",
      domainName: "d",
      hostname: "h",
      sid: "s",
      password: "p",
    });
    expect(mockInvoke).toHaveBeenCalledWith("registerUser", {
      userCode: "u",
      userName: "n",
      domainName: "d",
      hostname: "h",
      sid: "s",
      password: "p",
    });
  });

  it("updateUserCredential -> invoke('updateUserCredential', { userCode, password? })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateUserCredential({ userCode: "u", password: "p" });
    expect(mockInvoke).toHaveBeenCalledWith("updateUserCredential", {
      userCode: "u",
      password: "p",
    });
  });

  it("createUser -> invoke('createUser', { code, name, role })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createUser({ code: "u", name: "Alice", role: "admin" });
    expect(mockInvoke).toHaveBeenCalledWith("createUser", {
      code: "u",
      name: "Alice",
      role: "admin",
    });
  });

  it("listUsers -> invoke('listUsers')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listUsers();
    expect(mockInvoke).toHaveBeenCalledWith("listUsers");
  });

  it("getUserByCode -> invoke('getUserByCode', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getUserByCode("alice");
    expect(mockInvoke).toHaveBeenCalledWith("getUserByCode", { code: "alice" });
  });

  it("updateUser -> invoke('updateUser', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateUser("alice", { name: "Alicia" });
    expect(mockInvoke).toHaveBeenCalledWith("updateUser", {
      code: "alice",
      body: { name: "Alicia" },
    });
  });

  it("createProduct -> invoke('createProduct', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createProduct({ code: "p", name: "P", description: "" });
    expect(mockInvoke).toHaveBeenCalledWith("createProduct", {
      code: "p",
      name: "P",
      description: "",
    });
  });

  it("listProducts -> invoke('listProducts')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listProducts();
    expect(mockInvoke).toHaveBeenCalledWith("listProducts");
  });

  it("getProductByCode -> invoke('getProductByCode', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getProductByCode("p");
    expect(mockInvoke).toHaveBeenCalledWith("getProductByCode", { code: "p" });
  });

  it("updateProduct -> invoke('updateProduct', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateProduct("p", { active: false });
    expect(mockInvoke).toHaveBeenCalledWith("updateProduct", {
      code: "p",
      body: { active: false },
    });
  });

  it("createProject -> invoke('createProject', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createProject({ code: "p", description: "", productId: 1 });
    expect(mockInvoke).toHaveBeenCalledWith("createProject", {
      code: "p",
      description: "",
      productId: 1,
    });
  });

  it("listProjects -> invoke('listProjects')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listProjects();
    expect(mockInvoke).toHaveBeenCalledWith("listProjects");
  });

  it("getProjectByCode -> invoke('getProjectByCode', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getProjectByCode("p");
    expect(mockInvoke).toHaveBeenCalledWith("getProjectByCode", { code: "p" });
  });

  it("updateProject -> invoke('updateProject', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateProject("p", { active: false });
    expect(mockInvoke).toHaveBeenCalledWith("updateProject", {
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
