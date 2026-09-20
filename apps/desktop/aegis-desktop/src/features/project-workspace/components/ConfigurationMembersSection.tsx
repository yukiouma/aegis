import { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Chip,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { Close } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import {
  type ApiError,
  type ProjectMembersView,
  type UserSummary,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { useUpdateProject } from "../../project-list";
import { useListUsers } from "../../user";

export interface ConfigurationMembersSectionProps {
  projectCode: string;
  readonly: boolean;
  initial: ProjectMembersView;
}

/**
 * Members section: leaders rendered as read-only chips; workers
 * editable via Autocomplete + chip-with-X. Per-section Save mirrors
 * the General section: a single `membersTouched` flag flips on
 * first interaction, the body carries `members` only when touched.
 *
 * `members.leaders` is always re-sent back unchanged so a leader
 * change made through another surface (the project-list drawer,
 * etc.) survives this save.
 */
export function ConfigurationMembersSection({
  projectCode,
  readonly,
  initial,
}: ConfigurationMembersSectionProps) {
  const { t } = useI18n();
  const update = useUpdateProject();
  const users = useListUsers();

  const [leaders, setLeaders] = useState<UserSummary[]>(initial.leaders);
  const [workers, setWorkers] = useState<UserSummary[]>(initial.workers);
  const membersTouchedRef = useRef(false);

  const lastInitialRef = useRef(initial);
  useEffect(() => {
    if (lastInitialRef.current === initial) return;
    lastInitialRef.current = initial;
    setLeaders(initial.leaders);
    setWorkers(initial.workers);
    membersTouchedRef.current = false;
  }, [initial]);

  const touched = membersTouchedRef.current;
  const submitDisabled = readonly || !touched || update.isPending;

  // The autocomplete dropdown should exclude people already on the
  // team so the user can't pick the same code twice (duplicates are
  // rejected by the server anyway, but excluding them client-side
  // keeps the UX tidy).
  const usersExceptTeam = useMemo<UserSummary[]>(() => {
    const teamCodes = new Set([
      ...leaders.map((u) => u.code),
      ...workers.map((u) => u.code),
    ]);
    return (users.data ?? []).filter((u) => !teamCodes.has(u.code));
  }, [users.data, leaders, workers]);

  async function onSave() {
    await update.mutateAsync({
      code: projectCode,
      body: {
        members: {
          leaders: leaders.map((u) => u.code),
          workers: workers.map((u) => u.code),
        },
      },
    });
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
      <Box>
        <Typography variant="subtitle2" gutterBottom>
          {t("project.configuration.members.leadersHeading")}
        </Typography>
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ flexWrap: "wrap", gap: 0.5 }}
          data-testid="config-leaders"
        >
          {leaders.map((u) => (
            <Chip key={u.code} variant="outlined" label={`${u.code} — ${u.name}`} />
          ))}
          {leaders.length === 0 && (
            <Typography color="text.secondary">—</Typography>
          )}
        </Stack>
      </Box>

      <Box>
        <Typography variant="subtitle2" gutterBottom>
          {t("project.configuration.members.workersHeading")}
        </Typography>
        {!readonly && (
          <Autocomplete<UserSummary, true>
            multiple
            options={usersExceptTeam}
            getOptionLabel={(u) => `${u.code} — ${u.name}`}
            onChange={(_e, value) => {
              setWorkers(value);
              membersTouchedRef.current = true;
            }}
            renderInput={(params) => (
              <TextField
                {...params}
                size="small"
                placeholder={t("project.configuration.members.add")}
                inputProps={{
                  ...params.inputProps,
                  "data-testid": "config-workers-input",
                }}
              />
            )}
            sx={{ mb: 1 }}
          />
        )}
        <Stack
          direction="row"
          spacing={0.5}
          sx={{ flexWrap: "wrap", gap: 0.5 }}
          data-testid="config-workers"
        >
          {workers.map((u) =>
            readonly ? (
              <Chip
                key={u.code}
                variant="filled"
                label={`${u.code} — ${u.name}`}
              />
            ) : (
              <Chip
                key={u.code}
                variant="filled"
                label={`${u.code} — ${u.name}`}
                onDelete={() => {
                  setWorkers((prev) =>
                    prev.filter((w) => w.code !== u.code),
                  );
                  membersTouchedRef.current = true;
                }}
                deleteIcon={<Close />}
              />
            ),
          )}
          {workers.length === 0 && (
            <Typography color="text.secondary">
              {t("project.configuration.members.empty")}
            </Typography>
          )}
        </Stack>
      </Box>

      {!readonly && (
        <Stack direction="row" spacing={1} sx={{ justifyContent: "flex-end" }}>
          {update.error && (
            <Alert severity="error" sx={{ flexGrow: 1 }}>
              {t("project.configuration.saveFailed", {
                message: errorMessage(update.error as ApiError),
              })}
            </Alert>
          )}
          <Button
            variant="contained"
            disabled={submitDisabled}
            onClick={() => void onSave()}
            data-testid="config-members-save"
          >
            {t("common.save")}
          </Button>
        </Stack>
      )}
    </Box>
  );
}