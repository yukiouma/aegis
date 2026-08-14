import { useMemo, useState } from "react";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser, useListProjects } from "../data";
import type { ProjectView } from "../api";
import { ProjectDrawer } from "./ProjectDrawer";
import { ProjectFilterBar } from "./ProjectFilterBar";
import { ProjectTable } from "./ProjectTable";

interface DrawerState {
  mode: "closed" | "create" | "edit";
  code: string | null;
}

/**
 * Project list page. Owns the search / Involve filter state and the
 * drawer mode; passes filtered rows down to the table. Filters are
 * applied client-side as a single useMemo over the project list.
 */
export function ProjectListPage() {
  const { t } = useI18n();

  const projects = useListProjects();
  const currentUser = useCurrentUser();
  const currentCode = currentUser.data?.code ?? null;
  const role = currentUser.data?.role;
  const canEdit = role === "root" || role === "admin";

  const [query, setQuery] = useState("");
  const [involve, setInvolve] = useState(false);
  const [drawer, setDrawer] = useState<DrawerState>({
    mode: "closed",
    code: null,
  });

  const filteredRows = useMemo<ProjectView[]>(() => {
    const all = projects.data ?? [];
    const trimmed = query.trim();
    const q = trimmed.toLowerCase();
    return all.filter((row) => {
      // Search filter.
      if (q.length > 0) {
        const inCode = row.code.toLowerCase().includes(q);
        const inDescription = row.description.toLowerCase().includes(q);
        const inLeaders =
          leaderMatches(row.members.leaders, q) ||
          leaderMatches(row.unblindMembers.leaders, q);
        if (!inCode && !inDescription && !inLeaders) return false;
      }
      // Involve filter.
      if (involve && currentCode) {
        const inMembers =
          row.members.leaders.some((u) => u.code === currentCode) ||
          row.members.workers.some((u) => u.code === currentCode) ||
          row.unblindMembers.leaders.some((u) => u.code === currentCode) ||
          row.unblindMembers.workers.some((u) => u.code === currentCode);
        if (!inMembers) return false;
      }
      return true;
    });
  }, [projects.data, query, involve, currentCode]);

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4">{t("project.heading")}</Typography>

      <ProjectFilterBar
        query={query}
        onQueryChange={setQuery}
        involve={involve}
        onInvolveChange={setInvolve}
      />

      <ProjectTable
        rows={filteredRows}
        loading={projects.isLoading}
        error={projects.error}
        canEdit={canEdit}
        onOpenCreate={() => setDrawer({ mode: "create", code: null })}
        onOpenEdit={(code) => setDrawer({ mode: "edit", code })}
      />

      <ProjectDrawer
        mode={drawer.mode}
        code={drawer.code}
        onClose={() => setDrawer({ mode: "closed", code: null })}
      />
    </Box>
  );
}

function leaderMatches(
  leaders: { code: string; name: string }[],
  q: string,
): boolean {
  return leaders.some(
    (u) =>
      u.code.toLowerCase().includes(q) || u.name.toLowerCase().includes(q),
  );
}