import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Checkbox,
  Drawer,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useListUsers } from "../../user";
import { InitializeProjectFolderDialog } from "../../scaffold";
import {
  useCreateProject,
  useProject,
  useUpdateProject,
} from "../data/projects";
import {
  type ApiError,
  type CreateProjectInput,
  type ProjectConfiguration,
  type ProjectLanguage,
  type Tag,
  type UpdateProjectBody,
  type UserSummary,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { TagEditor } from "./TagEditor";

export interface ProjectDrawerProps {
  mode: "closed" | "create" | "edit";
  code: string | null;
  onClose: () => void;
  /** Called when the project was created but its folder could not be
   *  materialized. The drawer closes itself; the page surfaces the
   *  message, because the drawer is gone by then. */
  onFolderInitFailed?: (message: string) => void;
}

/**
 * Right-anchored MUI Drawer for create + update. The drawer body only
 * mounts when `mode !== "closed"` because the underlying Modal unmounts
 * children when `open={false}`. Edit mode triggers a one-shot
 * `get_project_by_code` fetch via `refetch()` to seed the form. The
 * `lookedUp` ref guards against React StrictMode double-fire. The
 * server's update endpoint treats a missing `tags` field as "leave
 * alone" and a present `tags` field as "replace whole list"; we honour
 * that by tracking `tagsTouched` and only including `tags` in the
 * update body when the user actually edited the editor in this
 * session.
 */
export function ProjectDrawer({
  mode,
  code,
  onClose,
  onFolderInitFailed,
}: ProjectDrawerProps) {
  const { t } = useI18n();

  const users = useListUsers();
  const fetched = useProject(code);
  const create = useCreateProject();
  const update = useUpdateProject();

  // Form state.
  const [formCode, setFormCode] = useState("");
  const [description, setDescription] = useState("");
  const [memberLeaders, setMemberLeaders] = useState<UserSummary[]>([]);
  const [memberWorkers, setMemberWorkers] = useState<UserSummary[]>([]);
  const [unblindLeaders, setUnblindLeaders] = useState<UserSummary[]>([]);
  const [unblindWorkers, setUnblindWorkers] = useState<UserSummary[]>([]);
  const [language, setLanguage] = useState<ProjectLanguage | null>(null);
  const [tags, setTags] = useState<Tag[]>([]);
  const [languageTouched, setLanguageTouched] = useState(false);
  const [tagsTouched, setTagsTouched] = useState(false);
  const [active, setActive] = useState(true);

  // Whether the created project's folder is materialized from a
  // structure template. On by default — most projects want their
  // folder on disk straight away.
  const [initializeFolder, setInitializeFolder] = useState(true);
  // The assembled create request, held while the template step is
  // open. It is non-null exactly while that step is open, and it
  // carries the code that both `create_project` and the scaffold
  // receive, so the two can never disagree.
  const [pendingInput, setPendingInput] = useState<CreateProjectInput | null>(
    null,
  );

  // Reset form to a clean slate when entering create mode. Depending
  // only on `mode` (not on the setters) means subsequent renders
  // while staying in "create" don't re-fire — only the transition
  // into "create" wipes the form. Without this, a previous edit
  // session's fields would bleed into the next create session
  // because the drawer component itself never unmounts.
  useEffect(() => {
    if (mode !== "create") return;
    setFormCode("");
    setDescription("");
    setMemberLeaders([]);
    setMemberWorkers([]);
    setUnblindLeaders([]);
    setUnblindWorkers([]);
    setLanguage(null);
    setTags([]);
    setLanguageTouched(false);
    setTagsTouched(false);
    setActive(true);
    setInitializeFolder(true);
    setPendingInput(null);
  }, [mode]);

  // Seed form when edit mode opens. The ref tracks which `code` we've
  // already seeded for, so StrictMode re-runs of the same code are a
  // no-op, but opening the drawer for a *different* project still
  // re-seeds. (`lookedUp: boolean` would persist across project
  // switches and freeze the form on the first project's data.)
  const lookedUpCode = useRef<string | null>(null);
  useEffect(() => {
    if (mode !== "edit" || code === null) return;
    if (lookedUpCode.current === code) return;
    lookedUpCode.current = code;
    void (async () => {
      const r = await fetched.refetch();
      if (r.isError || !r.data) return;
      setFormCode(r.data.code);
      setDescription(r.data.description);
      setMemberLeaders(r.data.members.leaders);
      setMemberWorkers(r.data.members.workers);
      setUnblindLeaders(r.data.unblindMembers.leaders);
      setUnblindWorkers(r.data.unblindMembers.workers);
      setLanguage(r.data.configurations.language);
      setTags(r.data.configurations.tags);
      setLanguageTouched(false);
      setTagsTouched(false);
      setActive(r.data.active);
    })();
  }, [mode, code, fetched]);

  const submitDisabled =
    !formCode.trim() ||
    !description.trim() ||
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
    const configurations: ProjectConfiguration = { language, tags };
    try {
      if (mode === "create") {
        const input: CreateProjectInput = {
          code: formCode.trim(),
          description: description.trim(),
          members,
          unblindMembers,
          configurations,
        };
        if (initializeFolder) {
          // The template step creates the project, in its `beforeInit`.
          setPendingInput(input);
          return;
        }
        await create.mutateAsync(input);
      } else if (mode === "edit" && code) {
        const body: UpdateProjectBody = {
          description: description.trim(),
          active,
          members,
          unblindMembers,
          ...(languageTouched || tagsTouched ? { configurations } : {}),
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
    <>
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

        <FormControl size="small" sx={{ maxWidth: 320 }}>
          <InputLabel id="project-language-label">
            {t("project.field.language")}
          </InputLabel>
          <Select<ProjectLanguage | "" >
            labelId="project-language-label"
            label={t("project.field.language")}
            value={language ?? ""}
            onChange={(e) => {
              const v = e.target.value;
              setLanguage(v === "" ? null : (v as ProjectLanguage));
              if (!languageTouched) setLanguageTouched(true);
            }}
          >
            <MenuItem value="">{t("project.field.language.none")}</MenuItem>
            <MenuItem value="en">{t("language.english")}</MenuItem>
            <MenuItem value="zh-CN">
              {t("language.simplifiedChinese")}
            </MenuItem>
          </Select>
        </FormControl>

        <TagEditor
          value={tags}
          onChange={setTags}
          onTouched={() => setTagsTouched(true)}
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

        {mode === "create" && (
          <Box>
            <FormControlLabel
              control={
                <Checkbox
                  checked={initializeFolder}
                  onChange={(event) =>
                    setInitializeFolder(event.target.checked)
                  }
                />
              }
              label={t("project.field.initializeFolder")}
            />
            <Typography variant="body2" color="text.secondary">
              {t("project.field.initializeFolder.hint")}
            </Typography>
          </Box>
        )}

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
    {/* A sibling of the Drawer, not a child: the Drawer unmounts its
        children when it closes, and the dialog has to outlive the
        drawer so the report stays on screen. */}
    {pendingInput && (
      <InitializeProjectFolderDialog
        open
        projectCode={pendingInput.code}
        beforeInit={async () => {
          await create.mutateAsync(pendingInput);
          // The project exists now — close the drawer, so a resubmit
          // cannot fail on the taken code.
          onClose();
        }}
        onInitError={(error) => {
          setPendingInput(null);
          onFolderInitFailed?.(
            t("project.folder.failed", { message: errorMessage(error) }),
          );
        }}
        onClose={() => setPendingInput(null)}
        onInitialized={() => {
          // Deliberately empty: the report the dialog just received is
          // rendered by the dialog itself and must stay on screen. The
          // drawer is already closed — `beforeInit` closed it.
        }}
      />
    )}
    </>
  );
}
