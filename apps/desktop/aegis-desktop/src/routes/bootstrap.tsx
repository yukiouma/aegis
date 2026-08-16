import { createFileRoute } from "@tanstack/react-router";

import { BootstrapPage } from "../features/bootstrap/pages/BootstrapPage";

export const Route = createFileRoute("/bootstrap")({
  component: BootstrapPage,
});
