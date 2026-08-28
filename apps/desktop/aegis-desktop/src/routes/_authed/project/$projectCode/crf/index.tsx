import { createFileRoute } from "@tanstack/react-router";

import { CrfFormListPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/",
)({
  component: CrfFormListPage,
});