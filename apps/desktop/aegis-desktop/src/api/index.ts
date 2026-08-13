import { invoke } from "@tauri-apps/api/core";

import type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  Identity,
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
  loginDomain: (): Promise<void> => call<void>("login_domain"),
  isLoggedIn: (): Promise<boolean> => call<boolean>("is_logged_in"),
  refresh: (): Promise<void> => call<void>("refresh"),
  logout: (): Promise<void> => call<void>("logout"),

  // identity
  getDomainUserInfo: (): Promise<Identity> =>
    call<Identity>("get_domain_user_info"),

  // user-credential
  registerUser: (input: RegisterUserInput): Promise<RegisterUserResponse> =>
    call<RegisterUserResponse>("register_user", { ...input }),
  updateUserCredential: (
    input: UpdateUserCredentialInput,
  ): Promise<UserCredentialView> =>
    call<UserCredentialView>("update_user_credential", { ...input }),

  // user
  createUser: (input: CreateUserInput): Promise<UserView> =>
    call<UserView>("create_user", { ...input }),
  listUsers: (): Promise<UserView[]> => call<UserView[]>("list_users"),
  getUserByCode: (code: string): Promise<UserView> =>
    call<UserView>("get_user_by_code", { code }),
  updateUser: (code: string, body: UpdateUserBody): Promise<UserView> =>
    call<UserView>("update_user", { code, body: { ...body } }),

  // product
  createProduct: (input: CreateProductInput): Promise<ProductView> =>
    call<ProductView>("create_product", { ...input }),
  listProducts: (): Promise<ProductView[]> => call<ProductView[]>("list_products"),
  getProductByCode: (code: string): Promise<ProductView> =>
    call<ProductView>("get_product_by_code", { code }),
  updateProduct: (code: string, body: UpdateProductBody): Promise<ProductView> =>
    call<ProductView>("update_product", { code, body: { ...body } }),

  // project
  createProject: (input: CreateProjectInput): Promise<ProjectView> =>
    call<ProjectView>("create_project", { ...input }),
  listProjects: (): Promise<ProjectView[]> => call<ProjectView[]>("list_projects"),
  getProjectByCode: (code: string): Promise<ProjectView> =>
    call<ProjectView>("get_project_by_code", { code }),
  updateProject: (code: string, body: UpdateProjectBody): Promise<ProjectView> =>
    call<ProjectView>("update_project", { code, body: { ...body } }),

  // health
  healthz: (): Promise<string> => call<string>("healthz"),
} as const;

export type { ApiError } from "./types";
export type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  Identity,
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
