import { createFileRoute } from "@tanstack/react-router";
import { ProjectDashboardPage } from "../../../../pages/ProjectDashboard";

export const Route = createFileRoute("/_authed/project/$projectCode/dashboard")({
  component: ProjectDashboardPage,
});