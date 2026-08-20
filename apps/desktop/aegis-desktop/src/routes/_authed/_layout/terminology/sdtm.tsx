import { createFileRoute } from "@tanstack/react-router";

import { TerminologyPage } from "../../../../features/terminology";

// `versionId` is preserved across navigation (e.g. drilling into a
// code list and using the back arrow) so the VersionDropdown keeps the
// user's selection. The same schema is mirrored on `adam.tsx`.
export const Route = createFileRoute("/_authed/_layout/terminology/sdtm")({
  // TanStack Router calls `validateSearch` more than once per
  // navigation — once with the previously parsed object (numbers
  // already coerced) and once with the raw URL params (everything a
  // string). Accept both so the dropdown's selection survives the
  // round-trip.
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
  component: () => <TerminologyPage kind="sdtm" />,
});