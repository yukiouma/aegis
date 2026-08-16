import { createFileRoute } from "@tanstack/react-router";
import { ProjectListPage } from "../../../pages/ProjectList";

export const Route = createFileRoute("/_authed/_layout/projects")({
  component: ProjectListPage,
});