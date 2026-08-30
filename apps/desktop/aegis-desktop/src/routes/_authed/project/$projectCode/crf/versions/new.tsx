import { createFileRoute } from "@tanstack/react-router";

import { CreateCrfVersionPage } from "../../../../../../features/crf";

export const Route = createFileRoute(
  "/_authed/project/$projectCode/crf/versions/new",
)({
  component: CreateCrfVersionPage,
});