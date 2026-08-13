import { createFileRoute, redirect } from "@tanstack/react-router";

import { api } from "../../api";
import { AppLayout } from "../../pages/layout";

export const Route = createFileRoute("/_layout")({
  // Every page under this layout requires a session. A failing
  // `is_logged_in` (a broken token store) counts as logged out, so the
  // user lands on the splash rather than seeing the router throw.
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
  component: AppLayout,
});
