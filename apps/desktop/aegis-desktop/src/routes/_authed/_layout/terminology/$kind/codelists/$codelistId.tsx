import { createFileRoute } from "@tanstack/react-router";
import type { TerminologyKind } from "../../../../../../shared/api";

import { CodeListDetailPage } from "../../../../../../features/terminology";

const KIND_VALUES: readonly TerminologyKind[] = ["sdtm", "adam"];

export const Route = createFileRoute(
  "/_authed/_layout/terminology/$kind/codelists/$codelistId",
)({
  parseParams: (raw) => ({
    kind: KIND_VALUES.includes(raw.kind as TerminologyKind)
      ? (raw.kind as TerminologyKind)
      : "sdtm",
    codelistId: Number(raw.codelistId),
  }),
  stringifyParams: ({ kind, codelistId }) => ({
    kind,
    codelistId: String(codelistId),
  }),
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
  component: CodeListDetailPage,
});