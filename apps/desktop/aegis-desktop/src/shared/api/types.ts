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
// (Product DTOs were removed from the server alongside the retired
//  Product aggregate; the desktop wire mirrors were dropped with it.)

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

// Terminology

export type TerminologyKind = "sdtm" | "adam";

export interface TerminologyVersionView {
  id: number;
  kind: TerminologyKind;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface TerminologyVersionListResponse {
  versions: TerminologyVersionView[];
}

export interface CreateTerminologyVersionInput {
  kind: TerminologyKind;
  name: string;
}

export interface UpdateTerminologyVersionInput {
  kind?: TerminologyKind;
  name?: string;
}

export interface CodeListView {
  id: number;
  versionId: number;
  code: string;
  extensible: boolean;
  name: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
  createdAt: string;
  updatedAt: string;
}

export interface CodeListListResponse {
  codelists: CodeListView[];
}

export interface PagedCodeListListResponse {
  items: CodeListView[];
  nextOffset?: number;
}

export interface CodeListListQuery {
  fragment?: string;
  offset?: number;
  limit?: number;
}

export interface CreateCodeListInput {
  versionId: number;
  code: string;
  extensible: boolean;
  name: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface UpdateCodeListInput {
  code?: string;
  extensible?: boolean;
  name?: string;
  submissionValue?: string;
  synonym?: string;
  definition?: string;
  nciPreferredTerm?: string;
}

export interface CodeItemView {
  id: number;
  codelistId: number;
  versionId: number;
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
  createdAt: string;
  updatedAt: string;
}

export interface CodeItemListResponse {
  items: CodeItemView[];
}

export interface PagedCodeItemListResponse {
  items: CodeItemView[];
  nextOffset?: number;
}

export interface CodeItemListQuery {
  fragment?: string;
  offset?: number;
  limit?: number;
}

export interface CreateCodeItemInput {
  codelistId: number;
  versionId: number;
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface UpdateCodeItemInput {
  code?: string;
  submissionValue?: string;
  synonym?: string;
  definition?: string;
  nciPreferredTerm?: string;
}

export interface BatchCodeItemEntry {
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface BatchCreateCodeItemsInput {
  codelistId: number;
  versionId: number;
  items: BatchCodeItemEntry[];
}

export interface BatchCreateCodeItemsResponse {
  count: number;
  codelistId: number;
  versionId: number;
}

// Domain model

export type DomainCategory =
  | "Special Purpose"
  | "Interventions"
  | "Events"
  | "Findings"
  | "Trial Design"
  | "Relationships"
  | "Study Reference";

export interface SdtmDomainDescriptionDetail {
  description: string;
  structure: string;
}

export interface SdtmDomainDescription {
  lang: string;
  details: SdtmDomainDescriptionDetail;
}

export interface SdtmDomainView {
  id: number;
  versionId: number;
  name: string;
  category: DomainCategory;
  descriptions: SdtmDomainDescription[];
  createdAt: string;
  updatedAt: string;
}

export interface SdtmVersionView {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface SdtmDomainListResponse {
  domains: SdtmDomainView[];
}

export interface SdtmVersionListResponse {
  versions: SdtmVersionView[];
}

// SDTM variables

export type SdtmVariableType = "Numeric" | "Character";
export type SdtmVariableCore = "Req" | "Exp" | "Perm" | "Supp";
export type SdtmRole =
  | "Identifier"
  | "Topic"
  | "Timing"
  | "Record Qualifier"
  | "Synonym Qualifier"
  | "Variable Qualifier"
  | "Grouping Qualifier"
  | "Rule";

export interface SdtmVariableDescriptionDetail {
  label: string;
}
export interface SdtmVariableDescription {
  lang: string;
  details: SdtmVariableDescriptionDetail;
}
export interface SdtmVariableView {
  id: number;
  domainId: number;
  name: string;
  variableControlled?: string;
  variableType: SdtmVariableType;
  variableCore: SdtmVariableCore;
  variableRole?: SdtmRole;
  variableSequence: number;
  descriptions: SdtmVariableDescription[];
  createdAt: string;
  updatedAt: string;
}
export interface SdtmVariableListResponse {
  variables: SdtmVariableView[];
}
export interface CreateSdtmVariableInput {
  domainId: number;
  name: string;
  variableControlled?: string;
  variableType: SdtmVariableType;
  variableCore: SdtmVariableCore;
  variableRole?: SdtmRole;
  variableSequence: number;
  descriptions: SdtmVariableDescription[];
}
// Three-state semantics: absent = no change, null = clear, value = replace.
export interface UpdateSdtmVariableInput {
  name?: string;
  variableControlled?: string | null;
  variableType?: SdtmVariableType;
  variableCore?: SdtmVariableCore;
  variableRole?: SdtmRole | null;
  variableSequence?: number;
  descriptions?: SdtmVariableDescription[];
}
export interface UpdateSdtmDomainInput {
  name?: string;
  category?: DomainCategory;
  descriptions?: SdtmDomainDescription[];
}
