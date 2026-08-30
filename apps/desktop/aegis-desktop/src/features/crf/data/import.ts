import { useMutation } from "@tanstack/react-query";

import { api } from "../../../shared/api";
import type { ApiError, CrfEdcType, CrfVersion } from "../../../shared/api";

export interface ImportAlsInput {
  name: string;
  projectCode: string;
  filepath: string;
  edcType: CrfEdcType;
}

export function useImportAls() {
  return useMutation<CrfVersion, ApiError, ImportAlsInput>({
    mutationFn: ({ name, projectCode, filepath, edcType }) =>
      api.importAls(name, projectCode, filepath, edcType),
  });
}