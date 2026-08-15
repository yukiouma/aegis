import { createFileRoute } from "@tanstack/react-router";

import { BootstrapPage } from "../pages/Bootstrap";

export const Route = createFileRoute("/bootstrap")({
  component: BootstrapPage,
});
