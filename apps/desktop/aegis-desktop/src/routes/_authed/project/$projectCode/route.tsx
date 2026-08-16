import { createFileRoute } from "@tanstack/react-router";

import { ProjectWorkspaceLayout } from "../../../../features/project-workspace/pages/ProjectWorkspaceLayout";

// UI shell for an individual project workspace (project-scoped sidebar + content).
// Auth is enforced by the parent `/_authed` layout, not here.
export const Route = createFileRoute("/_authed/project/$projectCode")({
  component: ProjectWorkspaceLayout,
});