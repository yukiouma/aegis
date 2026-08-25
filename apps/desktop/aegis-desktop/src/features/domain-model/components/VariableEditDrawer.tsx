import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Drawer,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { Add as AddIcon, Delete as DeleteIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  CreateSdtmVariableInput,
  SdtmRole,
  SdtmVariableCore,
  SdtmVariableDescription,
  SdtmVariableType,
  SdtmVariableView,
  UpdateSdtmVariableInput,
} from "../../../shared/api";

export interface VariableEditDrawerProps {
  open: boolean;
  mode: "create" | "edit";
  row?: SdtmVariableView;
  domainId: number;
  initialSequence?: number;
  onClose: () => void;
  onCreate: (input: CreateSdtmVariableInput) => void;
  onUpdate: (id: number, body: UpdateSdtmVariableInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const VARIABLE_TYPES: SdtmVariableType[] = ["Character", "Numeric"];
const VARIABLE_CORES: SdtmVariableCore[] = ["Req", "Exp", "Perm", "Supp"];
const VARIABLE_ROLES: (SdtmRole | null)[] = [
  null,
  "Identifier",
  "Topic",
  "Timing",
  "Record Qualifier",
  "Synonym Qualifier",
  "Variable Qualifier",
  "Grouping Qualifier",
  "Rule",
];

const EMPTY_DESCRIPTIONS: SdtmVariableDescription[] = [];

export function VariableEditDrawer({
  open,
  mode,
  row,
  domainId,
  initialSequence,
  onClose,
  onCreate,
  onUpdate,
  canMutate,
  mutationError,
  mutationPending,
}: VariableEditDrawerProps) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [variableControlled, setVariableControlled] = useState("");
  const [variableType, setVariableType] =
    useState<SdtmVariableType>("Character");
  const [variableCore, setVariableCore] = useState<SdtmVariableCore>("Req");
  const [variableRole, setVariableRole] = useState<SdtmRole | null>(null);
  const [descriptions, setDescriptions] = useState<SdtmVariableDescription[]>(
    [],
  );

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setName(row.name);
      setVariableControlled(row.variableControlled ?? "");
      setVariableType(row.variableType);
      setVariableCore(row.variableCore);
      setVariableRole(row.variableRole ?? null);
      setDescriptions(
        row.descriptions.length ? [...row.descriptions] : EMPTY_DESCRIPTIONS,
      );
    } else if (mode === "create") {
      setName("");
      setVariableControlled("");
      setVariableType("Character");
      setVariableCore("Req");
      setVariableRole(null);
      setDescriptions([]);
    }
  }, [open, mode, row]);

  function addDescription() {
    setDescriptions((d) => [...d, { lang: "", details: { label: "" } }]);
  }
  function updateDescription(
    idx: number,
    patch: Partial<SdtmVariableDescription>,
  ) {
    setDescriptions((d) =>
      d.map((item, i) => (i === idx ? { ...item, ...patch } : item)),
    );
  }
  function removeDescription(idx: number) {
    setDescriptions((d) => d.filter((_, i) => i !== idx));
  }

  function handleSubmit() {
    if (!canMutate) return;
    const trimmedName = name.trim();
    if (trimmedName === "") return;
    if (mode === "create") {
      onCreate({
        domainId,
        name: trimmedName,
        variableControlled:
          variableControlled.trim() === ""
            ? undefined
            : variableControlled,
        variableType,
        variableCore,
        variableRole: variableRole ?? undefined,
        variableSequence: initialSequence ?? 1,
        descriptions: descriptions.filter((d) => d.lang.trim() !== ""),
      });
    } else if (row) {
      const body: UpdateSdtmVariableInput = {
        name: trimmedName,
        variableType,
        variableCore,
        variableRole,
        descriptions: descriptions.filter((d) => d.lang.trim() !== ""),
      };
      // Only send variableControlled when it actually changed.
      const currentControlled = row.variableControlled ?? "";
      if (variableControlled.trim() === "" && currentControlled !== "") {
        body.variableControlled = null;
      } else if (variableControlled !== currentControlled) {
        body.variableControlled = variableControlled;
      }
      onUpdate(row.id, body);
    }
  }

  const title =
    mode === "create"
      ? t("domainModel.sdtm.variable.create.title")
      : t("domainModel.sdtm.variable.editTitle");
  const submitLabel =
    mode === "create" ? t("common.create") : t("common.save");

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">{title}</Typography>
        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("domainModel.sdtm.variable.field.name")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={!canMutate}
            required
          />
          <TextField
            size="small"
            label={t("domainModel.sdtm.variable.field.variableControlled")}
            value={variableControlled}
            onChange={(e) => setVariableControlled(e.target.value)}
            disabled={!canMutate}
          />
          <FormControl size="small" disabled={!canMutate}>
            <InputLabel id="variable-type-label">
              {t("domainModel.sdtm.variable.field.variableType")}
            </InputLabel>
            <Select
              labelId="variable-type-label"
              label={t("domainModel.sdtm.variable.field.variableType")}
              value={variableType}
              onChange={(e) =>
                setVariableType(e.target.value as SdtmVariableType)
              }
            >
              {VARIABLE_TYPES.map((vt) => (
                <MenuItem key={vt} value={vt}>
                  {t(`domainModel.sdtm.variable.type.${vt}`)}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <FormControl size="small" disabled={!canMutate}>
            <InputLabel id="variable-core-label">
              {t("domainModel.sdtm.variable.field.variableCore")}
            </InputLabel>
            <Select
              labelId="variable-core-label"
              label={t("domainModel.sdtm.variable.field.variableCore")}
              value={variableCore}
              onChange={(e) =>
                setVariableCore(e.target.value as SdtmVariableCore)
              }
            >
              {VARIABLE_CORES.map((vc) => (
                <MenuItem key={vc} value={vc}>
                  {t(`domainModel.sdtm.variable.core.${vc}`)}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <FormControl size="small" disabled={!canMutate}>
            <InputLabel id="variable-role-label">
              {t("domainModel.sdtm.variable.field.variableRole")}
            </InputLabel>
            <Select
              labelId="variable-role-label"
              label={t("domainModel.sdtm.variable.field.variableRole")}
              value={variableRole ?? "__null__"}
              onChange={(e) => {
                const v = e.target.value;
                setVariableRole(v === "__null__" ? null : (v as SdtmRole));
              }}
            >
              {VARIABLE_ROLES.map((vr) => (
                <MenuItem key={vr ?? "null"} value={vr ?? "__null__"}>
                  {vr ?? "—"}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              {t("domainModel.sdtm.variable.field.descriptions")}
            </Typography>
            <Stack spacing={1}>
              {descriptions.map((d, idx) => (
                <Stack
                  key={idx}
                  direction="row"
                  spacing={1}
                  alignItems="center"
                >
                  <TextField
                    size="small"
                    label={t("domainModel.sdtm.variable.field.descriptions.lang")}
                    value={d.lang}
                    onChange={(e) =>
                      updateDescription(idx, { lang: e.target.value })
                    }
                    disabled={!canMutate}
                    sx={{ width: 120 }}
                  />
                  <TextField
                    size="small"
                    label={t(
                      "domainModel.sdtm.variable.field.descriptions.label",
                    )}
                    value={d.details.label}
                    onChange={(e) =>
                      updateDescription(idx, {
                        details: { label: e.target.value },
                      })
                    }
                    disabled={!canMutate}
                    sx={{ flex: 1 }}
                  />
                  {canMutate && (
                    <IconButton
                      size="small"
                      aria-label="remove-description"
                      onClick={() => removeDescription(idx)}
                    >
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  )}
                </Stack>
              ))}
              {canMutate && (
                <Button
                  startIcon={<AddIcon />}
                  onClick={addDescription}
                  size="small"
                  sx={{ alignSelf: "flex-start" }}
                >
                  {t("domainModel.sdtm.variable.field.descriptions")}
                </Button>
              )}
            </Stack>
          </Box>
        </Stack>

        {mutationError && (
          <Alert severity="error">
            {t("domainModel.sdtm.detail.saveFailed", {
              message: errorMessage(mutationError),
            })}
          </Alert>
        )}

        <Box sx={{ display: "flex", gap: 1, justifyContent: "flex-end" }}>
          <Button onClick={onClose} disabled={mutationPending}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmit}
            disabled={!canMutate || name.trim() === "" || mutationPending}
          >
            {submitLabel}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}