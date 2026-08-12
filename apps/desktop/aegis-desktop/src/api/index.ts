import { invoke } from "@tauri-apps/api/core";

import type {
  ApiError,
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

export const api = {
  // auth
  login: (code: string, password: string): Promise<void> =>
    invoke<void>("login", { code, password }),
  loginDomain: (code: string): Promise<void> =>
    invoke<void>("loginDomain", { code }),
  isLoggedIn: (): Promise<boolean> => invoke<boolean>("isLoggedIn"),
  refresh: (): Promise<void> => invoke<void>("refresh"),
  logout: (): Promise<void> => invoke<void>("logout"),

  // user-credential
  registerUser: (input: RegisterUserInput): Promise<RegisterUserResponse> =>
    invoke<RegisterUserResponse>("registerUser", input),
  updateUserCredential: (
    input: UpdateUserCredentialInput,
  ): Promise<UserCredentialView> =>
    invoke<UserCredentialView>("updateUserCredential", input),

  // user
  createUser: (input: CreateUserInput): Promise<UserView> =>
    invoke<UserView>("createUser", input),
  listUsers: (): Promise<UserView[]> => invoke<UserView[]>("listUsers"),
  getUserByCode: (code: string): Promise<UserView> =>
    invoke<UserView>("getUserByCode", { code }),
  updateUser: (code: string, body: UpdateUserBody): Promise<UserView> =>
    invoke<UserView>("updateUser", { code, body }),

  // product
  createProduct: (input: CreateProductInput): Promise<ProductView> =>
    invoke<ProductView>("createProduct", input),
  listProducts: (): Promise<ProductView[]> => invoke<ProductView[]>("listProducts"),
  getProductByCode: (code: string): Promise<ProductView> =>
    invoke<ProductView>("getProductByCode", { code }),
  updateProduct: (code: string, body: UpdateProductBody): Promise<ProductView> =>
    invoke<ProductView>("updateProduct", { code, body }),

  // project
  createProject: (input: CreateProjectInput): Promise<ProjectView> =>
    invoke<ProjectView>("createProject", input),
  listProjects: (): Promise<ProjectView[]> => invoke<ProjectView[]>("listProjects"),
  getProjectByCode: (code: string): Promise<ProjectView> =>
    invoke<ProjectView>("getProjectByCode", { code }),
  updateProject: (code: string, body: UpdateProjectBody): Promise<ProjectView> =>
    invoke<ProjectView>("updateProject", { code, body }),

  // health
  healthz: (): Promise<string> => invoke<string>("healthz"),
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
