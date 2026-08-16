import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";

import { api } from "../../../../api";

// Pathful layout that gates every `/management/*` route on the current
// user's role. Only `root` and `admin` may enter; everyone else is
// bounced to the home page (they're authenticated, just not authorized
// for this section).
//
// Lives under `/_authed/_layout/management/` so the page still renders
// inside `AppLayout` (the sidebar). The parent `/_authed` already
// confirmed the user is logged in by the time we get here, so a
// failing `current_user` call counts as "not authorized" rather than
// "not logged in" — we redirect home, not to `/login`.
export const Route = createFileRoute("/_authed/_layout/management")({
  beforeLoad: async () => {
    let role: string | undefined;
    try {
      role = (await api.getCurrentUser()).role;
    } catch {
      role = undefined;
    }
    if (role !== "root" && role !== "admin") {
      throw redirect({ to: "/" });
    }
  },
  component: () => <Outlet />,
});
