import { createFileRoute } from "@tanstack/react-router";

import { AppLayout } from "../../../pages/Layout";

// UI shell for the top-level authenticated area (sidebar + content).
// Auth is enforced by the parent `/_authed` layout, not here.
export const Route = createFileRoute("/_authed/_layout")({
  component: AppLayout,
});