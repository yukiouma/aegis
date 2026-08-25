// Query key factory. Keys are tuples typed `as const` so downstream
// `useQuery({ queryKey: queryKeys.x.y() })` gets exact tuple inference.
// All hooks and invalidations reference keys through this module —
// never inline arrays — so a typo breaks one site at a time.

export const queryKeys = {
  auth: {
    loginStatus: () => ["auth", "loginStatus"] as const,
  },
  bootstrap: {
    health: () => ["bootstrap", "health"] as const,
  },
  user: {
    current: () => ["user", "current"] as const,
    domainIdentity: () => ["user", "domainIdentity"] as const,
    list: () => ["user", "list"] as const,
  },
  project: {
    all: () => ["project", "list"] as const,
    byCode: (code: string) => ["project", "byCode", code] as const,
  },
  product: {
    all: () => ["product", "list"] as const,
  },
  terminology: {
    versions: () => ["terminology", "versions"] as const,
    version: (id: number) => ["terminology", "version", id] as const,
    codeLists: (versionId: number, fragment: string) =>
      ["terminology", "codeLists", versionId, fragment] as const,
    codeList: (id: number) => ["terminology", "codeList", id] as const,
    codeItems: (codelistId: number, fragment: string) =>
      ["terminology", "codeItems", codelistId, fragment] as const,
    codeItemsByCode: (versionId: number, code: string) =>
      ["terminology", "codeItemsByCode", versionId, code] as const,
    codeItemsGlobal: (versionId: number, fragment: string) =>
      ["terminology", "codeItemsGlobal", versionId, fragment] as const,
  },
  domainModel: {
    sdtmVersions: () => ["domainModel", "sdtmVersions"] as const,
    sdtmDomains: (versionId: number) =>
      ["domainModel", "sdtmDomains", versionId] as const,
  },
} as const;