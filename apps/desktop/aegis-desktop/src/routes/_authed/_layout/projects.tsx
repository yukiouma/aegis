import { createFileRoute } from "@tanstack/react-router";
import { ProjectListPage } from "../../../features/project-list/pages/ProjectListPage";

export const Route = createFileRoute("/_authed/_layout/projects")({
  component: ProjectListPage,
});