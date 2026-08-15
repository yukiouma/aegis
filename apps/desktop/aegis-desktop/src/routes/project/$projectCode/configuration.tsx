import { createFileRoute } from "@tanstack/react-router";
import { ProjectConfigurationPage } from "../../../pages/ProjectConfiguration";

export const Route = createFileRoute("/project/$projectCode/configuration")({
  component: ProjectConfigurationPage,
});