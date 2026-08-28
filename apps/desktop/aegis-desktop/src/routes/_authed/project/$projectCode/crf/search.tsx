import { createFileRoute } from "@tanstack/react-router";

import { CrfGlobalSearchPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/search",
)({
  component: CrfGlobalSearchPage,
});