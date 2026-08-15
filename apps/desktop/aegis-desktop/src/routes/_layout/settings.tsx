import { createFileRoute } from "@tanstack/react-router";
import { SettingsPage } from "../../pages/Settings";

export const Route = createFileRoute("/_layout/settings")({
  component: SettingsPage,
});