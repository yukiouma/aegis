import { createFileRoute, redirect } from "@tanstack/react-router";

import { api } from "../../../api";
import { ProjectWorkspaceLayout } from "../../../pages/ProjectWorkspaceLayout";

export const Route = createFileRoute("/project/$projectCode")({
  // Every page under this layout requires a session. A failing
  // `is_logged_in` (a broken token store) counts as logged out, so the
  // user lands on the splash rather than seeing the router throw.
  // Same shape as `_layout/route.tsx` — duplicated rather than
  // factored out so each layout owns its own guard and the route is
  // self-contained.
  beforeLoad: async () => {
    let loggedIn = false;
    try {
      loggedIn = await api.isLoggedIn();
    } catch {
      loggedIn = false;
    }
    if (!loggedIn) {
      throw redirect({ to: "/login" });
    }
  },
  component: ProjectWorkspaceLayout,
});