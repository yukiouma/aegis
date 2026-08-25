import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  api,
  type ApiError,
  type SdtmDomainView,
  type SdtmVersionView,
} from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

export function useListSdtmVersions() {
  return useQuery<SdtmVersionView[], ApiError>({
    queryKey: queryKeys.domainModel.sdtmVersions(),
    queryFn: () => api.listSdtmVersions(),
  });
}

export function useListSdtmDomains(versionId: number | null) {
  return useQuery<SdtmDomainView[], ApiError>({
    queryKey: queryKeys.domainModel.sdtmDomains(versionId ?? 0),
    queryFn: () => api.listSdtmDomainsByVersion(versionId!),
    enabled: versionId != null && versionId > 0,
  });
}

export function useDeleteSdtmDomain() {
  const qc = useQueryClient();
  return useMutation<void, ApiError, number>({
    mutationFn: (id) => api.deleteSdtmDomain(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["domainModel", "sdtmDomains"] });
    },
  });
}