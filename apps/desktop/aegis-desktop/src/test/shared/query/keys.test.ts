import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import { queryKeys } from "../../../shared/query";

describe("queryKeys.mission.byProject", () => {
  // Regression guard: the cache key used by `useListMissionsByProject`
  // and the invalidation key used by the mission mutation hooks
  // (`useAddAssignee`, `useRemoveAssignee`, `useCreateMission`) must
  // be the same 4-element tuple. Earlier the factory took `kind?`
  // optional and produced a 4-element tuple with `undefined` at index
  // 3 — React Query's element-wise `===` prefix match then missed the
  // real cache entry (`undefined !== "crf"`) and the new/removed
  // assignee never appeared in the drawer or the table. The factory
  // now requires `kind`; this test pins the cache-key shape so the
  // kind cannot be silently dropped again.
  it("matches the cache key when both query and invalidation carry the same kind", () => {
    const qc = new QueryClient();
    const cacheKey = queryKeys.mission.byProject("AK001-002", "crf");
    qc.setQueryData(cacheKey, []);

    qc.invalidateQueries({ queryKey: cacheKey });

    expect(qc.getQueryState(cacheKey)?.isInvalidated).toBe(true);
  });
});