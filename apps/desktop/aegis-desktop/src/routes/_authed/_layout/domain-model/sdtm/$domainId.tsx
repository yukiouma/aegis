import { createFileRoute } from "@tanstack/react-router";

import { SdtmDomainDetail } from "../../../../../features/domain-model";

export const Route = createFileRoute(
  "/_authed/_layout/domain-model/sdtm/$domainId",
)({
  // `projectCode` scopes the terminology lookup: the drawer's code
  // items come from the SDTM terminology release the project is
  // configured against.
  validateSearch: (raw): { lang?: string; projectCode?: string } => ({
    lang:
      typeof raw.lang === "string" && raw.lang !== "" ? raw.lang : undefined,
    projectCode:
      typeof raw.projectCode === "string" && raw.projectCode !== ""
        ? raw.projectCode
        : undefined,
  }),
  component: () => <SdtmDomainDetail />,
});