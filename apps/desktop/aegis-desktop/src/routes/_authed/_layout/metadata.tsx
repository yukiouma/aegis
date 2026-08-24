import { createFileRoute } from "@tanstack/react-router";
import { MetadataPage } from "../../../features/metadata";

export const Route = createFileRoute("/_authed/_layout/metadata")({
  component: MetadataPage,
});