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
  },
} as const;