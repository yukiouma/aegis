import { useCallback, useMemo, useState } from "react";
import { Box } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser, useListUsers, useUpdateUser } from "../data";
import type { Role, UserView } from "../api";
import { UserFilterBar } from "./UserFilterBar";
import { UserTable } from "./UserTable";

/**
 * User management page. Lists non-root users with code, name, role,
 * and an active-state Switch. Filtering (root users hidden, search
 * by name/code) and the role gate live here; the table is a pure
 * presentational component.
 */
export function UserListPage() {
  const { t } = useI18n();
  const users = useListUsers();
  const currentUser = useCurrentUser();
  const updateUser = useUpdateUser();
  const [search, setSearch] = useState("");

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";
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

  if (!canManage) return null;

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