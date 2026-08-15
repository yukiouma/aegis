import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Drawer,
  FormControlLabel,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import {
  useCreateProject,
  useListProducts,
  useListUsers,
  useProject,
  useUpdateProject,
} from "../data";
import {
  type ApiError,
  type CreateProjectInput,
  type ProductView,
  type UpdateProjectBody,
  type UserSummary,
} from "../api";
import { errorMessage } from "../api/error";

export interface ProjectDrawerProps {
  mode: "closed" | "create" | "edit";
  code: string | null;
  onClose: () => void;
}

/**
 * Right-anchored MUI Drawer for create + update. The drawer body only
 * mounts when `mode !== "closed"` because the underlying Modal unmounts
 * children when `open={false}`. Edit mode triggers a one-shot
 * `get_project_by_code` fetch via `refetch()` to seed the form. The
 * `lookedUp` ref guards against React StrictMode double-fire.
 */
export function ProjectDrawer({ mode, code, onClose }: ProjectDrawerProps) {
  const { t } = useI18n();

  const products = useListProducts();
  const users = useListUsers();
  const fetched = useProject(code);
  const create = useCreateProject();
  const update = useUpdateProject();

  // Form state.
  const [formCode, setFormCode] = useState("");
  const [description, setDescription] = useState("");
  const [productId, setProductId] = useState<number | null>(null);
  const [memberLeaders, setMemberLeaders] = useState<UserSummary[]>([]);
  const [memberWorkers, setMemberWorkers] = useState<UserSummary[]>([]);
  const [unblindLeaders, setUnblindLeaders] = useState<UserSummary[]>([]);
  const [unblindWorkers, setUnblindWorkers] = useState<UserSummary[]>([]);
  const [active, setActive] = useState(true);

  // Seed form when edit mode opens. StrictMode-safe via `lookedUp` ref.
  const lookedUp = useRef(false);
  useEffect(() => {
    if (mode !== "edit" || code === null) return;
    if (lookedUp.current) return;
    lookedUp.current = true;
    void (async () => {
      const r = await fetched.refetch();
      if (r.isError || !r.data) return;
      setFormCode(r.data.code);
      setDescription(r.data.description);
      setProductId(r.data.product.id);
      setMemberLeaders(r.data.members.leaders);
      setMemberWorkers(r.data.members.workers);
      setUnblindLeaders(r.data.unblindMembers.leaders);
      setUnblindWorkers(r.data.unblindMembers.workers);
      setActive(r.data.active);
    })();
  }, [mode, code, fetched]);

  const submitDisabled =
    !formCode.trim() ||
    !description.trim() ||
    productId === null ||
    create.isPending ||
    update.isPending;

  async function onSubmit() {
    const members = {
      leaders: memberLeaders.map((u) => u.code),
      workers: memberWorkers.map((u) => u.code),
    };
    const unblindMembers = {
      leaders: unblindLeaders.map((u) => u.code),
      workers: unblindWorkers.map((u) => u.code),
    };
    try {
      if (mode === "create" && productId !== null) {
        const input: CreateProjectInput = {
          code: formCode.trim(),
          description: description.trim(),
          productId,
          members,
          unblindMembers,
        };
        await create.mutateAsync(input);
      } else if (mode === "edit" && code) {
        const body: UpdateProjectBody = {
          description: description.trim(),
          productId: productId ?? undefined,
          active,
          members,
          unblindMembers,
        };
        await update.mutateAsync({ code, body });
      }
      onClose();
    } catch {
      /* error surfaced below via create.error / update.error */
    }
  }

  const mutationError: ApiError | null =
    create.error ?? update.error ?? null;

  return (
    <Drawer
      anchor="right"
      open={mode !== "closed"}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t(mode === "create" ? "project.create.title" : "project.edit.title")}
        </Typography>

        <TextField
          label={t("project.field.code")}
          value={formCode}
          onChange={(event) => setFormCode(event.target.value)}
          disabled={mode === "edit"}
          size="small"
          required
        />

        <TextField
          label={t("project.field.description")}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          multiline
          minRows={2}
          size="small"
          required
        />

        <Autocomplete
          options={products.data ?? []}
          getOptionLabel={(p: ProductView) => `${p.code} — ${p.name}`}
          value={products.data?.find((p) => p.id === productId) ?? null}
          onChange={(_e, value: ProductView | null) =>
            setProductId(value?.id ?? null)
          }
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.product")}
              size="small"
              required
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={memberLeaders}
          onChange={(_e, value) => setMemberLeaders(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.members.leaders")}
              size="small"
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={memberWorkers}
          onChange={(_e, value) => setMemberWorkers(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.members.workers")}
              size="small"
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={unblindLeaders}
          onChange={(_e, value) => setUnblindLeaders(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.unblindMembers.leaders")}
              size="small"
            />
          )}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={unblindWorkers}
          onChange={(_e, value) => setUnblindWorkers(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.unblindMembers.workers")}
              size="small"
            />
          )}
        />

        {mode === "edit" && (
          <FormControlLabel
            control={
              <Switch
                checked={active}
                onChange={(event) => setActive(event.target.checked)}
              />
            }
            label={t("project.field.active")}
          />
        )}

        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}

        <Stack direction="row" spacing={1} sx={{ justifyContent: "flex-end" }}>
          <Button onClick={onClose}>{t("common.cancel")}</Button>
          <Button
            variant="contained"
            disabled={submitDisabled}
            onClick={() => void onSubmit()}
          >
            {t(mode === "create" ? "project.action.create" : "project.action.save")}
          </Button>
        </Stack>
      </Box>
    </Drawer>
  );
}