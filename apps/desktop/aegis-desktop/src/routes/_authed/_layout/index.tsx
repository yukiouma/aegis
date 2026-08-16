import { createFileRoute } from "@tanstack/react-router";
import { HomePage } from "../../../pages/Home";

export const Route = createFileRoute("/_authed/_layout/")({
  component: HomePage,
});