import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Chip,
  Drawer,
  IconButton,
  MenuItem,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { Close as CloseIcon, Delete as DeleteIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type {
  ApiError,
  CrfForm,
  MissionRole,
  MissionViewResponse,
  UserSummary,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import {
  useAddAssignee,
  useCreateMission,
  useRemoveAssignee,
} from "../../mission";
import { useProject } from "../../project-list";
import { useUserNameMap } from "../../user/data/list";

interface Props {
  open: boolean;
  row: CrfForm | null;
  projectCode: string;
  missions: MissionViewResponse[];
  onClose: () => void;
}

const ROLES: MissionRole[] = ["dev", "qc"];

/**
 * Drawer for assigning mission takers to a single CRF form.
 *
 * Workflow:
 *   1. Caller passes the existing missions list for the project.
 *   2. We pick the mission whose `missionCode` matches the form's code
 *      (or `undefined` if none).
 *   3. Add: if no mission exists yet, create one with the picked user
 *      as the first assignee. Otherwise add the user+role to the
 *      existing mission.
 *   4. Remove: delete the assignee by id from the matched mission.
 *
 * Membership for the user dropdown is sourced from the project's
 * `members` and `unblindMembers` (leaders + workers), minus the
 * users already assigned to the mission.
 */
export function CrfMissionAssignDrawer({
  open,
  row,
  projectCode,
  missions,
  onClose,
}: Props) {
  const { t } = useI18n();

  // Drive a one-shot lookup of the project so members are loaded.
  // Pass `enabled: open` so the query is active while the drawer is
  // open: when another surface (e.g. the project-list page) calls
  // `useUpdateProject` and invalidates `project.byCode(code)`, this
  // query auto-refetches and the user dropdown stays in sync. Drop
  // the previous manual `refetch()` effect — with `staleTime: 0`
  // and the query enabled, React Query fetches on mount itself.
  const projectQuery = useProject(projectCode, { enabled: open });
  const addAssignee = useAddAssignee(projectCode);
  const removeAssignee = useRemoveAssignee(projectCode);
  const createMission = useCreateMission(projectCode);
  const resolveName = useUserNameMap();

  const [userCode, setUserCode] = useState<string | null>(null);
  const [role, setRole] = useState<MissionRole>("dev");
  const [submitting, setSubmitting] = useState(false);

  const matched = useMemo<MissionViewResponse | undefined>(
    () => (row ? missions.find((m) => m.missionCode === row.code) : undefined),
    [missions, row],
  );

  // Project membership: dedupe across leaders/workers/unblind*.
  const members = useMemo<UserSummary[]>(() => {
    const project = projectQuery.data;
    if (!project) return [];
    const seen = new Map<string, UserSummary>();
    const collect = (list: UserSummary[] | undefined) => {
      for (const u of list ?? []) {
        if (!seen.has(u.code)) seen.set(u.code, u);
      }
    };
    collect(project.members?.leaders);
    collect(project.members?.workers);
    collect(project.unblindMembers?.leaders);
    collect(project.unblindMembers?.workers);
    return Array.from(seen.values());
  }, [projectQuery.data]);

  // Filter out users already on this mission.
  const availableMembers = useMemo<UserSummary[]>(() => {
    if (!matched) return members;
    const taken = new Set(matched.assignees.map((a) => a.userCode));
    return members.filter((m) => !taken.has(m.code));
  }, [members, matched]);

  // Reset local form state when the drawer re-opens with a different row.
  useEffect(() => {
    if (!open) return;
    setUserCode(null);
    setRole("dev");
    setSubmitting(false);
  }, [open, row?.id]);

  const mutationError: ApiError | null =
    addAssignee.error ??
    removeAssignee.error ??
    createMission.error ??
    null;

  const handleAdd = async () => {
    if (!row || !userCode) return;
    setSubmitting(true);
    try {
      if (matched) {
        await addAssignee.mutateAsync({
          missionId: matched.id,
          body: { userCode, role },
        });
      } else {
        await createMission.mutateAsync({
          projectCode,
          missionKind: "crf",
          missionCode: row.code,
          assignees: [{ userCode, role }],
        });
      }
      setUserCode(null);
    } catch {
      // surfaced via mutationError
    } finally {
      setSubmitting(false);
    }
  };

  const handleRemove = (assigneeId: number) => {
    if (!matched) return;
    void removeAssignee.mutate({ missionId: matched.id, assigneeId });
  };

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Box
          sx={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <Typography variant="h6">{t("crf.missionAssign.title")}</Typography>
          <IconButton
            size="small"
            aria-label={t("common.close")}
            onClick={onClose}
          >
            <CloseIcon />
          </IconButton>
        </Box>
        {row && (
          <Typography variant="subtitle2" color="text.secondary">
            {t("crf.missionAssign.heading", { code: row.code })}
          </Typography>
        )}

        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}

        <Box>
          {matched && matched.assignees.length > 0 ? (
            <Stack spacing={1}>
              {matched.assignees.map((a) => (
                <Box
                  key={a.id}
                  sx={{ display: "flex", alignItems: "center", gap: 1 }}
                >
                  <Typography sx={{ flexGrow: 1 }}>
                    {resolveName(a.userCode)}
                  </Typography>
                  <Chip
                    size="small"
                    label={
                      a.role === "qc"
                        ? t("crf.missionAssign.roleQc")
                        : t("crf.missionAssign.roleDev")
                    }
                    variant="outlined"
                    color="primary"
                    sx={
                      a.role === "qc"
                        ? { borderStyle: "dashed" }
                        : undefined
                    }
                  />
                  <IconButton
                    size="small"
                    color="error"
                    aria-label={t("crf.missionAssign.remove")}
                    onClick={() => handleRemove(a.id)}
                  >
                    <DeleteIcon fontSize="small" />
                  </IconButton>
                </Box>
              ))}
            </Stack>
          ) : (
            <Typography color="text.secondary" variant="body2">
              {t("crf.missionAssign.empty")}
            </Typography>
          )}
        </Box>

        <Box sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}>
          <Autocomplete
            sx={{ flex: 1 }}
            options={availableMembers}
            getOptionLabel={(opt) => opt.name}
            value={
              availableMembers.find((m) => m.code === userCode) ?? null
            }
            onChange={(_e, v) => setUserCode(v?.code ?? null)}
            isOptionEqualToValue={(opt, val) => opt.code === val.code}
            renderInput={(params) => (
              <TextField
                {...params}
                label={t("crf.missionAssign.field.user")}
                size="small"
              />
            )}
          />
          <TextField
            select
            size="small"
            label={t("crf.missionAssign.field.role")}
            value={role}
            onChange={(e) => setRole(e.target.value as MissionRole)}
            sx={{ width: 110 }}
          >
            {ROLES.map((r) => (
              <MenuItem key={r} value={r}>
                {r === "qc"
                  ? t("crf.missionAssign.roleQc")
                  : t("crf.missionAssign.roleDev")}
              </MenuItem>
            ))}
          </TextField>
        </Box>

        <Box sx={{ display: "flex", justifyContent: "flex-end", gap: 1 }}>
          <Button onClick={onClose}>{t("common.close")}</Button>
          <Button
            variant="contained"
            disabled={!userCode || submitting}
            onClick={() => void handleAdd()}
          >
            {t("crf.missionAssign.submit")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}