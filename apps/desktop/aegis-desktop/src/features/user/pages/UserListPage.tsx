import { useCallback, useMemo, useState } from "react";
import { Box } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser } from "../../auth";
import { useListUsers, useUpdateUser } from "../data/list";
import type { Role, UserView } from "../../../shared/api";
import { UserFilterBar } from "../components/UserFilterBar";
import { UserTable } from "../components/UserTable";

/**
 * User management page. Lists non-root users with code, name, role,
 * and an active-state Switch. Filtering (root users hidden, search
 * by name/code) lives here; the table is a pure presentational
 * component. Role authorization is enforced by the route guard at
 * `/_authed/_layout/management`, not here — by the time this
 * component mounts, the caller is already `root` or `admin`.
 */
export function UserListPage() {
  const { t } = useI18n();
  const users = useListUsers();
  const currentUser = useCurrentUser();
  const updateUser = useUpdateUser();
  const [search, setSearch] = useState("");

  const selfCode = currentUser.data?.code ?? null;

  // Trim + lowercase once so the memo dependency is a stable string
  // and trailing-whitespace-only edits don't trigger a re-render.
  const trimmedQuery = search.trim().toLowerCase();

  // Root users are never shown. Search (when present) filters by
  // case-insensitive substring on code OR name.
  const rows = useMemo<UserView[]>(() => {
    const list = (users.data ?? []).filter((u) => u.role !== "root");
    if (!trimmedQuery) return list;
    return list.filter(
      (u) =>
        u.code.toLowerCase().includes(trimmedQuery) ||
        u.name.toLowerCase().includes(trimmedQuery),
    );
  }, [users.data, trimmedQuery]);

  const handleToggle = useCallback(
    (code: string, nextActive: boolean) => {
      updateUser.mutate({ code, body: { active: nextActive } });
    },
    [updateUser],
  );

  const handleRoleChange = useCallback(
    (code: string, nextRole: Role) => {
      updateUser.mutate({ code, body: { role: nextRole } });
    },
    [updateUser],
  );

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <UserFilterBar query={search} onQueryChange={setSearch} />
      <UserTable
        rows={rows}
        loading={users.isLoading}
        mutationLoading={updateUser.isPending}
        error={users.error ?? updateUser.error}
        selfCode={selfCode}
        onToggle={handleToggle}
        onRoleChange={handleRoleChange}
        onRetry={users.refetch}
        emptyMessage={trimmedQuery ? t("user.noMatches") : undefined}
      />
    </Box>
  );
}