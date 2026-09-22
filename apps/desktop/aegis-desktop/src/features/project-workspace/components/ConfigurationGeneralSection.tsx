import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Stack,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import {
  type ApiError,
  type ProjectConfiguration,
  type ProjectConfigurationSdtmig,
  type ProjectLanguage,
  type Tag,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { useListSdtmVersions } from "../../domain-model/data/list";
import { useUpdateProject } from "../../project-list";
import { TagEditor } from "../../project-list/components/TagEditor";

export interface ConfigurationGeneralSectionProps {
  projectCode: string;
  readonly: boolean;
  initial: ProjectConfiguration;
}

/**
 * General section: language Select + TagEditor + per-section Save.
 * Mirrors `ProjectDrawer`'s "touched" semantics — `languageTouched`
 * and `tagsTouched` flip independently on the user's first
 * interaction, and the body carries `configurations` only when at
 * least one is true. Touched flags survive across renders but
 * reset on every re-mount, so a re-mount with new `initial` data
 * (e.g. after a successful save) starts a fresh session.
 *
 * `readonly` short-circuits the Save button entirely: a non-leader
 * sees the controls but can't edit them, so there's nothing to save.
 */
export function ConfigurationGeneralSection({
  projectCode,
  readonly,
  initial,
}: ConfigurationGeneralSectionProps) {
  const { t } = useI18n();
  const update = useUpdateProject();

  const [language, setLanguage] = useState<ProjectLanguage | null>(
    initial.language,
  );
  const [tags, setTags] = useState<Tag[]>(initial.tags);
  const [sdtmig, setSdtmig] = useState<ProjectConfigurationSdtmig | null>(
    initial.sdtmig ?? null,
  );
  const languageTouchedRef = useRef(false);
  const tagsTouchedRef = useRef(false);
  const sdtmTouchedRef = useRef(false);

  const versions = useListSdtmVersions();

  // Reseed the local state whenever a fresh `initial` arrives
  // (e.g. after `useUpdateProject` invalidates the project cache
  // and the parent's hook re-fetches).
  const lastInitialRef = useRef(initial);
  useEffect(() => {
    if (lastInitialRef.current === initial) return;
    lastInitialRef.current = initial;
    setLanguage(initial.language);
    setTags(initial.tags);
    setSdtmig(initial.sdtmig ?? null);
    languageTouchedRef.current = false;
    tagsTouchedRef.current = false;
    sdtmTouchedRef.current = false;
  }, [initial]);

  const dirty =
    languageTouchedRef.current ||
    tagsTouchedRef.current ||
    sdtmTouchedRef.current;
  const submitDisabled = readonly || !dirty || update.isPending;

  async function onSave() {
    const configurations: ProjectConfiguration = { language, tags, sdtmig };
    await update.mutateAsync({
      code: projectCode,
      body: { configurations },
    });
    // The mutation invalidates the project cache; the parent's
    // `useProjectConfiguration` will refetch and our `lastInitialRef`
    // effect will reseed the local state, clearing the touched flags.
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
      <FormControl size="small" disabled={readonly}>
        <InputLabel id="config-language-label">
          {t("project.configuration.general.language")}
        </InputLabel>
        <Select<ProjectLanguage | "" >
          labelId="config-language-label"
          label={t("project.configuration.general.language")}
          value={language ?? ""}
          onChange={(e) => {
            const v = e.target.value;
            setLanguage(v === "" ? null : (v as ProjectLanguage));
            languageTouchedRef.current = true;
          }}
          inputProps={{ "data-testid": "config-language-input" }}
        >
          <MenuItem value="">
            {t("project.configuration.general.language.none")}
          </MenuItem>
          <MenuItem value="en">{t("language.english")}</MenuItem>
          <MenuItem value="zh-CN">
            {t("language.simplifiedChinese")}
          </MenuItem>
        </Select>
      </FormControl>

      <FormControl size="small" disabled={readonly}>
        <InputLabel id="config-sdtmig-label">
          {t("project.configuration.general.sdtmig")}
        </InputLabel>
        <Select<string>
          labelId="config-sdtmig-label"
          label={t("project.configuration.general.sdtmig")}
          value={sdtmig ? String(sdtmig.versionId) : ""}
          onChange={(e) => {
            const v = e.target.value;
            if (v === "") {
              setSdtmig(null);
            } else {
              const id = Number(v);
              const found = versions.data?.find((x) => x.id === id);
              setSdtmig({ versionId: id, versionName: found?.name ?? "" });
            }
            sdtmTouchedRef.current = true;
          }}
          inputProps={{ "data-testid": "config-sdtmig-input" }}
        >
          <MenuItem value="">
            {t("project.configuration.general.sdtmig.none")}
          </MenuItem>
          {/* Synthetic MenuItem for the seeded selection when the
              version list hasn't resolved yet — MUI Select otherwise
              warns about an out-of-range value. */}
          {sdtmig &&
            !versions.data?.some((v) => v.id === sdtmig.versionId) && (
              <MenuItem value={String(sdtmig.versionId)}>
                {sdtmig.versionName}
              </MenuItem>
            )}
          {(versions.data ?? []).map((v) => (
            <MenuItem key={v.id} value={String(v.id)}>
              {v.name}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      <Box>
        <InputLabel sx={{ mb: 1 }}>
          {t("project.configuration.general.tags")}
        </InputLabel>
        <TagEditor
          value={tags}
          onChange={setTags}
          onTouched={() => {
            tagsTouchedRef.current = true;
          }}
        />
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
            data-testid="config-general-save"
          >
            {t("common.save")}
          </Button>
        </Stack>
      )}
    </Box>
  );
}