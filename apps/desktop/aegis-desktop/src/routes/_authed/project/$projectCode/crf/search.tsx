import { createFileRoute } from "@tanstack/react-router";

import { CrfGlobalSearchPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/search",
)({
  validateSearch: (raw): { versionId?: number } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === ""
          ? undefined
          : Number(raw.versionId)
        : typeof raw.versionId === "number"
          ? raw.versionId
          : undefined,
  }),
  component: CrfGlobalSearchPage,
});