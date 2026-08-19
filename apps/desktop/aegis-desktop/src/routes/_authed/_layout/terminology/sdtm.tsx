import { createFileRoute } from "@tanstack/react-router";

import { TerminologyPage } from "../../../../features/terminology";

export const Route = createFileRoute("/_authed/_layout/terminology/sdtm")({
  component: () => <TerminologyPage kind="sdtm" />,
});