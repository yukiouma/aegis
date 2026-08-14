import { createFileRoute } from "@tanstack/react-router";
import { ProjectListPage } from "../../pages/project-list";

export const Route = createFileRoute("/_layout/projects")({
  component: ProjectListPage,
});