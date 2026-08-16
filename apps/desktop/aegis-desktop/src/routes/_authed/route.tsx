import { createFileRoute, Outlet, redirect } from "@tanstack/react-router";

import { api } from "../../api";

// Pathless layout that owns the auth guard for every page below it. Adding
// any new authenticated route is now a matter of placing it under
// `src/routes/_authed/` — the guard lives here, once.
//
// A failing `is_logged_in` (e.g. a broken token store) counts as logged out,
// so the user lands on the splash rather than seeing the router throw.
export const Route = createFileRoute("/_authed")({
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
  component: () => <Outlet />,
});