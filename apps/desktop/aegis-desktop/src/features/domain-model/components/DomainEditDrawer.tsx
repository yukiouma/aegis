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
  CreateSdtmDomainInput,
  DomainCategory,
  SdtmDomainDescription,
  SdtmDomainView,
  UpdateSdtmDomainInput,
} from "../../../shared/api";

export interface DomainEditDrawerProps {
  open: boolean;
  row: SdtmDomainView;
  mode?: "create" | "edit";
  versionId?: number;
  onClose: () => void;
  onUpdate: (id: number, body: UpdateSdtmDomainInput) => void;
  onCreate?: (input: CreateSdtmDomainInput) => void;
  canMutate: boolean;
  mutationError: ApiError | null;
  mutationPending: boolean;
}

const CATEGORIES: DomainCategory[] = [
  "Special Purpose",
  "Interventions",
  "Events",
  "Findings",
  "Trial Design",
  "Relationships",
  "Study Reference",
];

const EMPTY_DESCRIPTIONS: SdtmDomainDescription[] = [];

export function DomainEditDrawer({
  open,
  row,
  mode = "edit",
  versionId,
  onClose,
  onUpdate,
  onCreate,
  canMutate,
  mutationError,
  mutationPending,
}: DomainEditDrawerProps) {
  const { t } = useI18n();
  const [name, setName] = useState(row.name);
  const [category, setCategory] = useState<DomainCategory>(row.category);
  const [descriptions, setDescriptions] = useState<SdtmDomainDescription[]>(
    row.descriptions.length ? [...row.descriptions] : EMPTY_DESCRIPTIONS,
  );

  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      setName("");
      setCategory("Special Purpose");
      setDescriptions(EMPTY_DESCRIPTIONS);
    } else {
      setName(row.name);
      setCategory(row.category);
      setDescriptions(
        row.descriptions.length ? [...row.descriptions] : EMPTY_DESCRIPTIONS,
      );
    }
  }, [open, mode, row]);

  function addDescription() {
    setDescriptions((d) => [
      ...d,
      { lang: "", details: { description: "", structure: "" } },
    ]);
  }
  function updateDescription(
    idx: number,
    patch: Partial<SdtmDomainDescription>,
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
    const trimmed = name.trim();
    if (trimmed === "") return;
    if (mode === "create") {
      if (versionId == null || onCreate == null) return;
      onCreate({
        versionId,
        name: trimmed,
        category,
        descriptions: descriptions.filter((d) => d.lang.trim() !== ""),
      });
      return;
    }
    const body: UpdateSdtmDomainInput = {
      name: trimmed,
      category,
      descriptions: descriptions.filter((d) => d.lang.trim() !== ""),
    };
    onUpdate(row.id, body);
  }

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 850 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {mode === "create"
            ? t("domainModel.sdtm.create.title")
            : t("domainModel.sdtm.detail.editTitle")}
        </Typography>
        <Stack spacing={2}>
          <TextField
            size="small"
            label={t("project.field.code")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={!canMutate}
            required
          />
          <FormControl size="small" disabled={!canMutate}>
            <InputLabel id="domain-category-label">
              {t("domainModel.sdtm.col.category")}
            </InputLabel>
            <Select
              labelId="domain-category-label"
              label={t("domainModel.sdtm.col.category")}
              value={category}
              onChange={(e) => setCategory(e.target.value as DomainCategory)}
            >
              {CATEGORIES.map((c) => (
                <MenuItem key={c} value={c}>
                  {c}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              {t("domainModel.sdtm.col.description")}
            </Typography>
            <Stack spacing={1}>
              {descriptions.map((d, idx) => (
                <Stack
                  key={idx}
                  direction="row"
                  spacing={1}
                  sx={{ alignItems: "flex-start" }}
                >
                  <TextField
                    size="small"
                    label="Lang"
                    value={d.lang}
                    onChange={(e) =>
                      updateDescription(idx, { lang: e.target.value })
                    }
                    disabled={!canMutate}
                    sx={{ width: 100 }}
                  />
                  <TextField
                    size="small"
                    label={t("domainModel.sdtm.col.description")}
                    value={d.details.description}
                    onChange={(e) =>
                      updateDescription(idx, {
                        details: { ...d.details, description: e.target.value },
                      })
                    }
                    disabled={!canMutate}
                    sx={{ flex: 1 }}
                  />
                  <TextField
                    size="small"
                    label={t("domainModel.sdtm.col.structure")}
                    value={d.details.structure}
                    onChange={(e) =>
                      updateDescription(idx, {
                        details: { ...d.details, structure: e.target.value },
                      })
                    }
                    disabled={!canMutate}
                    sx={{ flex: 1 }}
                  />
                  {canMutate && (
                    <IconButton
                      color="error"
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
                  {t("domainModel.sdtm.col.description")}
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
            disabled={
              !canMutate ||
              name.trim() === "" ||
              (mode === "create" && versionId == null) ||
              mutationPending
            }
          >
            {mode === "create" ? t("common.create") : t("common.save")}
          </Button>
        </Box>
      </Box>
    </Drawer>
  );
}