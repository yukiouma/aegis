import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
  type InfiniteData,
  type QueryKey,
} from "@tanstack/react-query";

import {
  api,
  type ApiError,
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
