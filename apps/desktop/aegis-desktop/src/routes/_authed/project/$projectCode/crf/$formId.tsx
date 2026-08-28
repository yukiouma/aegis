import { createFileRoute } from "@tanstack/react-router";

import { CrfDetailPage } from "../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/$formId",
)({
  component: CrfDetailPage,
});