import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import type {
  CodeItemListQuery,
  CodeItemListResponse,
  CodeItemView,
  CodeListListQuery,
  CodeListView,
  CreateCodeItemInput,
  CreateCodeListInput,
  CreateProjectInput,
  CreateTerminologyVersionInput,
  CreateUserInput,
  Identity,
  PagedCodeItemListResponse,
  PagedCodeListListResponse,
  ProjectView,
  RegisterUserInput,
  RegisterUserResponse,
  TerminologyKind,
  TerminologyVersionView,
  UpdateCodeItemInput,
  UpdateCodeListInput,
  UpdateProjectBody,
  UpdateTerminologyVersionInput,
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
  getCurrentUser: (): Promise<UserView> =>
    call<UserView>("current_user"),
  updateUser: (code: string, body: UpdateUserBody): Promise<UserView> =>
    call<UserView>("update_user", { code, body: { ...body } }),

  // product
  // (Product CRUD was removed from the server alongside the retired
  //  Product aggregate; the desktop wrappers were dropped with it.)

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

  // workspace window
  openProjectWorkspace: async (code: string): Promise<void> => {
    const label = `project:${code}`;
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }
    new WebviewWindow(label, {
      url: `/project/${code}`,
      title: code,
      width: 1100,
      height: 720,
      minWidth: 720,
      minHeight: 480,
      maximized: true,
    });
  },

  // terminology
  listTerminologyVersions: (): Promise<TerminologyVersionView[]> =>
    call<TerminologyVersionView[]>("list_terminology_versions"),
  createTerminologyVersion: (
    input: CreateTerminologyVersionInput,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("create_terminology_version", { ...input }),
  getTerminologyVersionById: (id: number): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("get_terminology_version_by_id", { id }),
  updateTerminologyVersion: (
    id: number,
    body: UpdateTerminologyVersionInput,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("update_terminology_version", {
      id,
      body: { ...body },
    }),
  deleteTerminologyVersion: (id: number): Promise<void> =>
    call<void>("delete_terminology_version", { id }),

  listCodeLists: (
    versionId: number,
    options: CodeListListQuery = {},
  ): Promise<PagedCodeListListResponse> =>
    call<PagedCodeListListResponse>("list_code_lists", {
      versionId,
      fragment: options.fragment,
      offset: options.offset,
      limit: options.limit,
    }),
  getCodeListById: (id: number): Promise<CodeListView> =>
    call<CodeListView>("get_code_list_by_id", { id }),
  createCodeList: (input: CreateCodeListInput): Promise<CodeListView> =>
    call<CodeListView>("create_code_list", { ...input }),
  updateCodeList: (
    id: number,
    body: UpdateCodeListInput,
  ): Promise<CodeListView> =>
    call<CodeListView>("update_code_list", { id, body: { ...body } }),
  deleteCodeList: (id: number): Promise<void> =>
    call<void>("delete_code_list", { id }),

  listCodeItems: (
    codelistId: number,
    options: CodeItemListQuery = {},
  ): Promise<PagedCodeItemListResponse> =>
    call<PagedCodeItemListResponse>("list_code_items", {
      codelistId,
      fragment: options.fragment,
      offset: options.offset,
      limit: options.limit,
    }),
  listCodeItemsByVersionAndCode: (
    versionId: number,
    code: string,
  ): Promise<CodeItemListResponse> =>
    call<CodeItemListResponse>("list_code_items_by_version_and_code", {
      versionId,
      code,
    }),
  createCodeItem: (input: CreateCodeItemInput): Promise<CodeItemView> =>
    call<CodeItemView>("create_code_item", { ...input }),
  updateCodeItem: (
    id: number,
    body: UpdateCodeItemInput,
  ): Promise<CodeItemView> =>
    call<CodeItemView>("update_code_item", { id, body: { ...body } }),
  deleteCodeItem: (id: number): Promise<void> =>
    call<void>("delete_code_item", { id }),

  importTerminology: (
    kind: TerminologyKind,
    filepath: string,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("import_terminology", { kind, filepath }),
} as const;

export type { ApiError } from "./types";
export type {
  CodeItemListQuery,
  CodeItemListResponse,
  CodeItemView,
  CodeListListQuery,
  CodeListListResponse,
  CodeListView,
  CreateCodeItemInput,
  CreateCodeListInput,
  CreateProjectInput,
  CreateTerminologyVersionInput,
  CreateUserInput,
  Identity,
  PagedCodeItemListResponse,
  PagedCodeListListResponse,
  ProjectMembers,
  ProjectMembersView,
  ProjectView,
  Role,
  RegisterUserInput,
  RegisterUserResponse,
  Tag,
  TerminologyKind,
  TerminologyVersionListResponse,
  TerminologyVersionView,
  UpdateCodeItemInput,
  UpdateCodeListInput,
  UpdateProjectBody,
  UpdateTerminologyVersionInput,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserSummary,
  UserView,
} from "./types";
