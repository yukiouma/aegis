import { createFileRoute } from "@tanstack/react-router";
import { ProjectDashboardPage } from "../../../../features/project-workspace/pages/ProjectDashboardPage";

export const Route = createFileRoute("/_authed/project/$projectCode/dashboard")({
  component: ProjectDashboardPage,
});