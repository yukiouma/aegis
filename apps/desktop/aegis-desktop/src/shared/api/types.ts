// Wire-DTO mirrors. Hand-maintained — every shape matches the Rust DTO in
// `apps/desktop/aegis-desktop/src-tauri/src/http/*` 1:1.
//
// Field naming: TypeScript interfaces use camelCase identifiers. Note that
// this is purely a TS-style rename — the actual JSON keys received from
// the aegis-server are snake_case (per the server's wire format). Future
// consumers that destructure these shapes need to know they must use the
// snake_case keys at runtime, OR a transform layer must be added.

export type Role = "root" | "admin" | "general";

export interface ErrorBody {
  code: string;
  message: string;
}

// Mirrors `http::dto::ApiError`. The Rust enum uses struct variants
// (serde tagged enums forbid newtype variants) tagged with `kind =
// "camelCase"`, so multi-word variants serialize as `refreshFailed` etc.
export type ApiError =
  | { kind: "network"; message: string }
  | { kind: "http"; status: number; code: string; message: string }
  | { kind: "refreshFailed" }
  | { kind: "notImplemented"; detail: string }
  | { kind: "store"; message: string };

// Mirrors `system::identity::Identity` in src-tauri. That struct carries
// `#[serde(rename_all = "camelCase")]`, so unlike the other response
// shapes in this file its JSON keys really are camelCase — `hostMachine`,
// not `host_machine`.
export interface Identity {
  domain: string;
  hostMachine: string;
  sid: string;
  userid: string;
}

// Auth
export interface RegisterUserInput {
  userCode: string;
  userName: string;
  domainName: string;
  hostname: string;
  sid: string;
  password: string;
}
export interface RegisterUserResponse {
  userCode: string;
  userName: string;
  role: Role;
  active: boolean;
  domainName: string;
  hostname: string;
  sid: string;
}
export interface UserCredentialView {
  userCode: string;
  passwordHash: string;
  tokenVersion: number;
}
export interface UpdateUserCredentialInput {
  userCode: string;
  password?: string;
}

// User
export interface UserView {
  id: number;
  code: string;
  name: string;
  role: Role;
  active: boolean;
  createdAt: string;
  updatedAt: string;
}
export interface CreateUserInput {
  code: string;
  name: string;
  role: Role;
}
export interface UpdateUserBody {
  code?: string;
  name?: string;
  role?: Role;
  active?: boolean;
}

// Product
export interface ProductView {
  id: number;
  code: string;
  name: string;
  description: string;
  active: boolean;
  createdAt: string;
  updatedAt: string;
}
export interface CreateProductInput {
  code: string;
  name: string;
  description: string;
}
export interface UpdateProductBody {
  code?: string;
  name?: string;
  description?: string;
  active?: boolean;
}

// Project
export interface UserSummary {
  code: string;
  name: string;
}
export interface ProjectMembers {
  leaders?: string[];
  workers?: string[];
}
export interface ProjectMembersView {
  leaders: UserSummary[];
  workers: UserSummary[];
}
export interface Tag {
  key: string;
  value: string;
}
export interface ProjectView {
  id: number;
  code: string;
  description: string;
  members: ProjectMembersView;
  unblindMembers: ProjectMembersView;
  tags: Tag[];
  active: boolean;
  createdAt: string;
  updatedAt: string;
}
export interface CreateProjectInput {
  code: string;
  description: string;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  tags?: Tag[];
}
export interface UpdateProjectBody {
  code?: string;
  description?: string;
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  tags?: Tag[];
}
