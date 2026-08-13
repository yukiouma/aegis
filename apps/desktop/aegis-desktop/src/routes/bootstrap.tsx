import { createFileRoute } from "@tanstack/react-router";

import { BootstrapPage } from "../pages/bootstrap";

export const Route = createFileRoute("/bootstrap")({
  component: BootstrapPage,
});
