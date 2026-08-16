import { createFileRoute } from "@tanstack/react-router";

import { UserListPage } from "../../../../pages/UserList";

export const Route = createFileRoute("/_authed/_layout/management/users")({
  component: UserListPage,
});