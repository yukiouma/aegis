// Wire-DTO mirrors. Hand-maintained — every shape matches the Rust DTO in
// `apps/desktop/aegis-desktop/src-tauri/src/http/*` 1:1.

export type Role = "root" | "admin" | "general";

export interface ErrorBody {
  code: string;
  message: string;
}

// Mirrors `http::dto::ApiError`. The Rust enum uses struct variants (serde
// tagged enums forbid newtype variants), so each variant shape carries a
// named field next to its `kind` discriminator.
export type ApiError =
  | { kind: "network"; message: string }
  | { kind: "http"; status: number; code: string; message: string }
  | { kind: "refresh_failed" }
  | { kind: "not_implemented"; detail: string }
  | { kind: "store"; message: string };

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
  user_code: string;
  user_name: string;
  role: Role;
  active: boolean;
  domain_name: string;
  hostname: string;
  sid: string;
}
export interface UserCredentialView {
  user_code: string;
  password_hash: string;
  token_version: number;
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
  created_at: string;
  updated_at: string;
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
  created_at: string;
  updated_at: string;
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
export interface ProjectView {
  id: number;
  code: string;
  description: string;
  product: ProductView;
  members: ProjectMembersView;
  unblind_members: ProjectMembersView;
  active: boolean;
  created_at: string;
  updated_at: string;
}
export interface CreateProjectInput {
  code: string;
  description: string;
  productId: number;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
}
export interface UpdateProjectBody {
  code?: string;
  description?: string;
  productId?: number;
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
}
