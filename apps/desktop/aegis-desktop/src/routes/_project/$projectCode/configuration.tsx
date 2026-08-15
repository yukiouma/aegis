import { createFileRoute } from "@tanstack/react-router";
import { ProjectConfigurationPage } from "../../../pages/ProjectConfiguration";

export const Route = createFileRoute(
  "/_project/$projectCode/configuration",
)({
  component: ProjectConfigurationPage,
});