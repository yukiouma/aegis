import { createFileRoute } from "@tanstack/react-router";

import { ProjectConfigurationPage } from "../../../../features/project-workspace/pages/ProjectConfigurationPage";

export const Route = createFileRoute("/_authed/project/$projectCode/configuration")({
  component: ProjectConfigurationPage,
});