import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_authed/project/$projectCode/")({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: "/project/$projectCode/dashboard",
      params: { projectCode: params.projectCode },
    });
  },
});