import { invoke } from "@tauri-apps/api/core";

import type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  ProductView,
  ProjectView,
  RegisterUserInput,
  RegisterUserResponse,
  UpdateProductBody,
  UpdateProjectBody,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserView,
} from "./types";

// Thin wrapper that loosens the `args` parameter type from
// `InvokeArgs` (= `Record<string, unknown>`) to `unknown` so typed input
// interfaces (CreateUserInput, etc.) flow through without per-call casts.
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (args === undefined) {
    return invoke<T>(cmd);
  }
  return invoke<T>(cmd, args);
}

export const api = {
  // auth
  login: (code: string, password: string): Promise<void> =>
    call<void>("login", { code, password }),
  loginDomain: (code: string): Promise<void> =>
    call<void>("loginDomain", { code }),
  isLoggedIn: (): Promise<boolean> => call<boolean>("isLoggedIn"),
  refresh: (): Promise<void> => call<void>("refresh"),
  logout: (): Promise<void> => call<void>("logout"),

  // user-credential
  registerUser: (input: RegisterUserInput): Promise<RegisterUserResponse> =>
    call<RegisterUserResponse>("registerUser", { ...input }),
  updateUserCredential: (
    input: UpdateUserCredentialInput,
  ): Promise<UserCredentialView> =>
    call<UserCredentialView>("updateUserCredential", { ...input }),

  // user
  createUser: (input: CreateUserInput): Promise<UserView> =>
    call<UserView>("createUser", { ...input }),
  listUsers: (): Promise<UserView[]> => call<UserView[]>("listUsers"),
  getUserByCode: (code: string): Promise<UserView> =>
    call<UserView>("getUserByCode", { code }),
  updateUser: (code: string, body: UpdateUserBody): Promise<UserView> =>
    call<UserView>("updateUser", { code, body: { ...body } }),

  // product
  createProduct: (input: CreateProductInput): Promise<ProductView> =>
    call<ProductView>("createProduct", { ...input }),
  listProducts: (): Promise<ProductView[]> => call<ProductView[]>("listProducts"),
  getProductByCode: (code: string): Promise<ProductView> =>
    call<ProductView>("getProductByCode", { code }),
  updateProduct: (code: string, body: UpdateProductBody): Promise<ProductView> =>
    call<ProductView>("updateProduct", { code, body: { ...body } }),

  // project
  createProject: (input: CreateProjectInput): Promise<ProjectView> =>
    call<ProjectView>("createProject", { ...input }),
  listProjects: (): Promise<ProjectView[]> => call<ProjectView[]>("listProjects"),
  getProjectByCode: (code: string): Promise<ProjectView> =>
    call<ProjectView>("getProjectByCode", { code }),
  updateProject: (code: string, body: UpdateProjectBody): Promise<ProjectView> =>
    call<ProjectView>("updateProject", { code, body: { ...body } }),

  // health
  healthz: (): Promise<string> => call<string>("healthz"),
} as const;

export type { ApiError } from "./types";
export type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  ProductView,
  ProjectMembers,
  ProjectMembersView,
  ProjectView,
  Role,
  RegisterUserInput,
  RegisterUserResponse,
  UpdateProductBody,
  UpdateProjectBody,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserSummary,
  UserView,
} from "./types";
