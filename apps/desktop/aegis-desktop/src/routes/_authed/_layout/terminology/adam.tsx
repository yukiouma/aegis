import { createFileRoute } from "@tanstack/react-router";

import { TerminologyPage } from "../../../../features/terminology";

// Mirror of `sdtm.tsx`: same `versionId` search schema so the
// VersionDropdown survives navigating into a code list and back.
export const Route = createFileRoute("/_authed/_layout/terminology/adam")({
  // Mirror of `sdtm.tsx`: accept both `string` (raw URL params) and
  // `number` (already-parsed values from TanStack Router's multiple
  // `validateSearch` invocations).
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
  component: () => <TerminologyPage kind="adam" />,
});