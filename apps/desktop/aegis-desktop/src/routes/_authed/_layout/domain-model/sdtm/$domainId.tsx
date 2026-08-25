import { createFileRoute } from "@tanstack/react-router";

import { SdtmDomainDetail } from "../../../../../features/domain-model";

export const Route = createFileRoute(
  "/_authed/_layout/domain-model/sdtm/$domainId",
)({
  validateSearch: (raw): { lang?: string } => ({
    lang:
      typeof raw.lang === "string" && raw.lang !== "" ? raw.lang : undefined,
  }),
  component: () => <SdtmDomainDetail />,
});