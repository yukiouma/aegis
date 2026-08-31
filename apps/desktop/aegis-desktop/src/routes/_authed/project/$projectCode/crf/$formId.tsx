import { createFileRoute } from "@tanstack/react-router";

import { CrfDetailPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/$formId",
)({
  validateSearch: (raw): { versionId?: number; focus?: string } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === ""
          ? undefined
          : Number(raw.versionId)
        : typeof raw.versionId === "number"
          ? raw.versionId
          : undefined,
    focus:
      typeof raw.focus === "string" && raw.focus !== ""
        ? raw.focus
        : undefined,
  }),
  component: CrfDetailPage,
});