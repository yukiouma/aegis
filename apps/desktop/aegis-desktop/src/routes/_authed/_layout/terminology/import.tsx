import { createFileRoute } from "@tanstack/react-router";
import type { TerminologyKind } from "../../../../shared/api";

import { ImportTerminologyPage } from
  "../../../../features/terminology/pages/ImportTerminologyPage";

export const Route = createFileRoute(
  "/_authed/_layout/terminology/import",
)({
  validateSearch: (raw): { kind?: TerminologyKind } => ({
    kind:
      raw.kind === "sdtm" || raw.kind === "adam" ? raw.kind : undefined,
  }),
  component: ImportTerminologyPage,
});