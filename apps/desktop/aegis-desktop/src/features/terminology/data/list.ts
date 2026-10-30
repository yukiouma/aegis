import {
  useInfiniteQuery,
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
  type InfiniteData,
  type QueryKey,
} from "@tanstack/react-query";
import { useMemo } from "react";

import {
  api,
  type ApiError,
  type CodeItemListResponse,
  type CodeItemView,
  type CodeListView,
  type CreateCodeItemInput,
  type CreateCodeListInput,
  type CreateTerminologyVersionInput,
  type PagedCodeItemListResponse,
  type PagedCodeListListResponse,
  type TerminologyVersionView,
  type UpdateCodeItemInput,
  type UpdateCodeListInput,
  type UpdateTerminologyVersionInput,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";
import { useProject } from "../../project-list";

/** Page size used by both code-list and code-item tables. */
export const PAGE_SIZE = 20;

// ---- Versions ----

export function useListTerminologyVersions() {
  return useQuery<TerminologyVersionView[], ApiError>({
    queryKey: queryKeys.terminology.versions(),
    queryFn: () => api.listTerminologyVersions(),
  });
}

export function useCreateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<
    TerminologyVersionView,
    ApiError,
    CreateTerminologyVersionInput
  >({
    mutationFn: api.createTerminologyVersion,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useUpdateTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<
    TerminologyVersionView,
    ApiError,
    { id: number; body: UpdateTerminologyVersionInput }
  >({
    mutationFn: ({ id, body }) => api.updateTerminologyVersion(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

export function useDeleteTerminologyVersion() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: api.deleteTerminologyVersion,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
    },
  });
}

// ---- Code lists ----

export interface ListPagedOptions {
  fragment?: string;
}

/**
 * Codelists for a given terminology version. Uses `useInfiniteQuery` so the
 * caller can `fetchNextPage()` to append more rows without losing the
 * previously-fetched ones. Each page is a `PagedCodeListListResponse`; the
 * next page's offset is read from `lastPage.nextOffset`. `fragment = ""`
 * (or whitespace) is treated as "no filter" by stripping it before sending.
 */
export function useListCodeLists(
  versionId: number | null,
  options: ListPagedOptions = {},
) {
  const fragment = options.fragment ?? "";
  return useInfiniteQuery<PagedCodeListListResponse, ApiError, InfiniteData<PagedCodeListListResponse>, QueryKey, number>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0, fragment),
    queryFn: ({ pageParam }) =>
      api.listCodeLists(versionId!, {
        fragment: fragment.trim() === "" ? undefined : fragment,
        offset: pageParam,
        limit: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) => lastPage.nextOffset,
    enabled: versionId != null && versionId > 0,
  });
}

export function useGetCodeList(id: number | null) {
  return useQuery<CodeListView, ApiError>({
    queryKey: queryKeys.terminology.codeList(id ?? 0),
    queryFn: () => api.getCodeListById(id!),
    enabled: id != null && id > 0,
  });
}

export function useCreateCodeList() {
  const qc = useQueryClient();
  return useMutation<CodeListView, ApiError, CreateCodeListInput>({
    mutationFn: api.createCodeList,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeLists", created.versionId],
      });
    },
  });
}

export function useUpdateCodeList() {
  const qc = useQueryClient();
  return useMutation<
    CodeListView,
    ApiError,
    { id: number; body: UpdateCodeListInput }
  >({
    mutationFn: ({ id, body }) => api.updateCodeList(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeLists", updated.versionId],
      });
      qc.invalidateQueries({
        queryKey: queryKeys.terminology.codeList(updated.id),
      });
    },
  });
}

export function useDeleteCodeList() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; versionId: number }
  >({
    mutationFn: ({ id }) => api.deleteCodeList(id),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeLists", vars.versionId],
      });
    },
  });
}

// ---- Code items ----

export function useListCodeItems(
  codelistId: number | null,
  options: ListPagedOptions = {},
) {
  const fragment = options.fragment ?? "";
  return useInfiniteQuery<PagedCodeItemListResponse, ApiError, InfiniteData<PagedCodeItemListResponse>, QueryKey, number>({
    queryKey: queryKeys.terminology.codeItems(codelistId ?? 0, fragment),
    queryFn: ({ pageParam }) =>
      api.listCodeItems(codelistId!, {
        fragment: fragment.trim() === "" ? undefined : fragment,
        offset: pageParam,
        limit: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) => lastPage.nextOffset,
    enabled: codelistId != null && codelistId > 0,
  });
}

/**
 * Version-scoped code-list search for the global term search page.
 * Shares the `codeLists` query key with `useListCodeLists`, so a search
 * query caches identically to a list query.
 */
export function useSearchCodeLists(
  versionId: number | null,
  options: ListPagedOptions = {},
) {
  const fragment = options.fragment ?? "";
  return useInfiniteQuery<PagedCodeListListResponse, ApiError, InfiniteData<PagedCodeListListResponse>, QueryKey, number>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0, fragment),
    queryFn: ({ pageParam }) =>
      api.listCodeLists(versionId!, {
        fragment: fragment.trim() === "" ? undefined : fragment,
        offset: pageParam,
        limit: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) => lastPage.nextOffset,
    enabled: versionId != null && versionId > 0,
  });
}

/**
 * Cross-codelist code-item search scoped to a single version for the
 * global term search page. Uses a dedicated `codeItemsGlobal` key to avoid
 * collisions with the per-codelist `codeItems` cache. `versionId` is sent
 * to the server; `codelistId` is omitted (null) so the server applies
 * only the version filter.
 */
export function useSearchCodeItems(
  versionId: number | null,
  options: ListPagedOptions = {},
) {
  const fragment = options.fragment ?? "";
  return useInfiniteQuery<PagedCodeItemListResponse, ApiError, InfiniteData<PagedCodeItemListResponse>, QueryKey, number>({
    queryKey: queryKeys.terminology.codeItemsGlobal(versionId ?? 0, fragment),
    queryFn: ({ pageParam }) =>
      api.listCodeItems(null, {
        versionId,
        fragment: fragment.trim() === "" ? undefined : fragment,
        offset: pageParam,
        limit: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) => lastPage.nextOffset,
    enabled: versionId != null && versionId > 0,
  });
}

export function useCreateCodeItem() {
  const qc = useQueryClient();
  return useMutation<CodeItemView, ApiError, CreateCodeItemInput>({
    mutationFn: api.createCodeItem,
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeItems", created.codelistId],
      });
    },
  });
}

export function useUpdateCodeItem() {
  const qc = useQueryClient();
  return useMutation<
    CodeItemView,
    ApiError,
    { id: number; body: UpdateCodeItemInput }
  >({
    mutationFn: ({ id, body }) => api.updateCodeItem(id, body),
    onSuccess: (updated) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeItems", updated.codelistId],
      });
    },
  });
}

export function useDeleteCodeItem() {
  const qc = useQueryClient();
  return useMutation<
    void,
    ApiError,
    { id: number; codelistId: number }
  >({
    mutationFn: ({ id }) => api.deleteCodeItem(id),
    onSuccess: (_void, vars) => {
      qc.invalidateQueries({
        queryKey: ["terminology", "codeItems", vars.codelistId],
      });
    },
  });
}

/**
 * Single-page lookup of all code items sharing a given code within a
 * terminology version. The server endpoint is non-paginated, so we use
 * `useQuery` instead of `useInfiniteQuery`. The hook is `enabled` only when
 * both `versionId` and `code` are usable; the dialog flips `code` to a
 * real value to trigger the fetch.
 */
export function useListCodeItemsByVersionAndCode(
  versionId: number | null,
  code: string | null,
) {
  return useQuery<CodeItemListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeItemsByCode(versionId ?? 0, code ?? ""),
    queryFn: () => api.listCodeItemsByVersionAndCode(versionId!, code!),
    enabled: versionId != null && versionId > 0 && !!code,
  });
}

/**
 * Bulk lookup of codelists by id. Returns one `UseQueryResult` per id in
 * the same order. React Query dedupes by `queryKey`, so overlapping ids
 * across dialog opens share their cache entry with the single-id
 * `useGetCodeList` hook.
 *
 * `staleTime: 30_000` keeps the dialog snappy on re-open within 30s.
 */
export function useGetCodeListsByIds(ids: number[]) {
  return useQueries({
    queries: ids.map((id) => ({
      queryKey: queryKeys.terminology.codeList(id),
      queryFn: () => api.getCodeListById(id),
      enabled: id > 0,
      staleTime: 30_000,
    })),
  });
}

/**
 * The SDTM terminology version a terminology code should be resolved
 * against.
 *
 * Resolution rules:
 *   1. When a project is in scope and it configures an SDTM
 *      terminology release, use that release. A stale id whose name
 *      still matches a known release resolves by name — the drawer is
 *      read-only, so the stored pointer is not healed.
 *   2. Otherwise — no project, no configured release, or one that
 *      matches nothing — use the most recently imported SDTM release
 *      (the highest id; the server lists versions by id ascending).
 *
 * While the project is still loading the version is `null`: resolving
 * to the fallback first would fire the code-list lookup against the
 * wrong version.
 */
export interface SdtmTerminologyVersion {
  versionId: number | null;
  isLoading: boolean;
  /** Error from the terminology versions query. */
  versionsError: ApiError | null;
  /** Error from the project query — only when a project is in scope. */
  projectError: ApiError | null;
  refetch: () => void;
}

export function useSdtmTerminologyVersionId(
  projectCode: string | null,
): SdtmTerminologyVersion {
  const versionsQuery = useListTerminologyVersions();
  const projectQuery = useProject(projectCode, {
    enabled: projectCode != null && projectCode !== "",
  });

  const versions = versionsQuery.data ?? [];
  const configured =
    projectQuery.data?.configurations.sdtmTerminology ?? null;
  const projectPending = projectCode != null && projectQuery.isLoading;

  const versionId = useMemo(() => {
    if (projectPending) return null;
    const sdtmVersions = versions.filter((v) => v.kind === "sdtm");
    if (configured) {
      const byId = sdtmVersions.find((v) => v.id === configured.versionId);
      if (byId) return byId.id;
      const byName = sdtmVersions.find(
        (v) => v.name.toLowerCase() === configured.versionName.toLowerCase(),
      );
      if (byName) return byName.id;
    }
    return sdtmVersions.reduce<number | null>(
      (latest, v) => (latest == null || v.id > latest ? v.id : latest),
      null,
    );
  }, [versions, configured, projectPending]);

  return {
    versionId,
    isLoading:
      versionsQuery.isLoading ||
      (projectCode != null && projectQuery.isLoading),
    versionsError: versionsQuery.error,
    projectError: projectQuery.error,
    refetch: () => {
      void versionsQuery.refetch();
      if (projectCode != null) void projectQuery.refetch();
    },
  };
}

/**
 * Find the code list whose `name` equals `name` in a terminology
 * version. The server search is a full-text match, so several rows can
 * come back; the exact name match wins, and a case-insensitive match is
 * accepted as a fallback (the search itself is case-insensitive).
 *
 * The lookup asks for the server's maximum page size so a common token
 * does not push the exact match off the first page. It uses its own
 * query key — `codeLists` belongs to the 20-per-page infinite query.
 */
export function useFindCodeListByName(
  versionId: number | null,
  name: string | null,
) {
  const query = useQuery<PagedCodeListListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeListByName(versionId ?? 0, name ?? ""),
    queryFn: () =>
      api.listCodeLists(versionId!, { fragment: name!, offset: 0, limit: 500 }),
    enabled: versionId != null && versionId > 0 && !!name,
  });
  const codelist = useMemo(() => {
    const items = query.data?.items ?? [];
    if (!name) return null;
    return (
      items.find((c) => c.name === name) ??
      items.find((c) => c.name.toLowerCase() === name.toLowerCase()) ??
      null
    );
  }, [query.data, name]);
  return { ...query, codelist };
}
