import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { queryKeys } from "../../../shared/query";

describe("queryKeys.mission.byProject", () => {
  it("matches the kind-bearing cache key via prefix when kind is omitted", () => {
    const qc = new QueryClient();
    // Seed the cache with the kind-bearing key (what
    // `useListMissionsByProject` writes).
    const cacheKey = queryKeys.mission.byProject("AK001-002", "crf");
    qc.setQueryData(cacheKey, []);

    // Invalidate with the kind-less key (what the mutations call).
    qc.invalidateQueries({
      queryKey: queryKeys.mission.byProject("AK001-002"),
    });

    // If the factory's tuple length and `undefined` last element
    // round-trip through React Query's prefix-match correctly, the
    // cache entry is now invalidated. If not (the bug we're
    // chasing), it stays untouched.
    expect(qc.getQueryState(cacheKey)?.isInvalidated).toBe(true);
  });
});