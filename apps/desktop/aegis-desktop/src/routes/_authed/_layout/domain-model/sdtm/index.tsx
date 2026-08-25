import { createFileRoute } from "@tanstack/react-router";

import { SdtmDomainList } from "../../../../../features/domain-model";

// `versionId` and `lang` are preserved across navigation so the
// dropdowns keep the user's selection. Mirror the terminology/sdtm.tsx
// validateSearch pattern.
export const Route = createFileRoute("/_authed/_layout/domain-model/sdtm/")({
  validateSearch: (raw): { versionId?: number; lang?: string } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === ""
          ? undefined
          : Number(raw.versionId)
        : typeof raw.versionId === "number"
          ? raw.versionId
          : undefined,
    lang:
      typeof raw.lang === "string" && raw.lang !== "" ? raw.lang : undefined,
  }),
  component: SdtmDomainList,
});