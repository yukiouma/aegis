import { createFileRoute } from "@tanstack/react-router";

import { SplashPage } from "../pages/splash";

export const Route = createFileRoute("/splash")({
  component: SplashPage,
});
