import { createFileRoute } from "@tanstack/react-router";
import { ProjectDashboardPage } from "../../../pages/ProjectDashboard";

export const Route = createFileRoute("/project/$projectCode/dashboard")({
  component: ProjectDashboardPage,
});