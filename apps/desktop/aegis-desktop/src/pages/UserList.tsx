import { useCallback, useMemo } from "react";
import { Box, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useCurrentUser, useListUsers, useUpdateUser } from "../data";
import type { Role, UserView } from "../api";
import { UserTable } from "./UserTable";

/**
 * User management page. Lists non-root users with code, name, role,
 * and an active-state Switch. Filtering (root users hidden) and the
 * role gate live here; the table is a pure presentational component.
 */
export function UserListPage() {
  const { t } = useI18n();
  const users = useListUsers();
  const currentUser = useCurrentUser();
  const updateUser = useUpdateUser();

  const role = currentUser.data?.role;
  const canManage = role === "root" || role === "admin";
  const selfCode = currentUser.data?.code ?? null;

  // Root users are never shown. Filter is single-pass over the list.
  const rows = useMemo<UserView[]>(
    () => (users.data ?? []).filter((u) => u.role !== "root"),
    [users.data],
  );

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
      <Typography variant="h4">{t("user.heading")}</Typography>
      <UserTable
        rows={rows}
        loading={users.isLoading}
        mutationLoading={updateUser.isPending}
        error={users.error ?? updateUser.error}
        selfCode={selfCode}
        onToggle={handleToggle}
        onRoleChange={handleRoleChange}
        onRetry={users.refetch}
      />
    </Box>
  );
}