import { useEffect, useRef } from "react";
import {
  Box,
  Button,
  IconButton,
  Stack,
  TextField,
} from "@aegis/ui/mui";
import { Add, Close } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { Tag } from "../../../shared/api";

export interface TagEditorProps {
  value: Tag[];
  onChange: (next: Tag[]) => void;
  onTouched?: () => void;
}

/**
 * Tag editor. Pure controlled component: parent's `value` /
 * `onChange` is the source of truth. Rows render top-to-bottom in
 * `value` order. After `append`, the new key TextField is focused
 * (managed via a single `useRef<number>` + `useEffect` chain —
 * resize-on-reseed). `onTouched` is fired at most once per render
 * cycle so a parent's "edited?" flag flips on the first interaction
 * only.
 */
export function TagEditor({ value, onChange, onTouched }: TagEditorProps) {
  const { t } = useI18n();

  // Track which row was just appended so the useEffect can focus it.
  const lastAppendedIndex = useRef<number>(-1);
  const keyInputRefs = useRef<(HTMLInputElement | null)[]>([]);
  // onTouched is one-shot: it flips the parent's "edited?" flag once
  // and stays flipped, so subsequent interactions must not refire it.
  const touchedFiredRef = useRef<boolean>(false);

  // Reset focus bookkeeping when the parent's value length changes
  // (e.g. drawer re-seeds with new tags from the wire).
  useEffect(() => {
    keyInputRefs.current.length = value.length;
  }, [value.length]);

  // Focus the newly-appended key input, then clear the pointer so a
  // later render doesn't steal focus back.
  useEffect(() => {
    if (lastAppendedIndex.current >= 0) {
      const target = keyInputRefs.current[lastAppendedIndex.current];
      target?.focus();
      lastAppendedIndex.current = -1;
    }
  });

  function emit(next: Tag[]) {
    onChange(next);
    if (!touchedFiredRef.current) {
      touchedFiredRef.current = true;
      onTouched?.();
    }
  }

  function updateRow(index: number, patch: Partial<Tag>) {
    const next = value.map((row, i) => (i === index ? { ...row, ...patch } : row));
    emit(next);
  }

  function removeRow(index: number) {
    emit(value.filter((_, i) => i !== index));
  }

  function appendRow() {
    emit([...value, { key: "", value: "" }]);
    lastAppendedIndex.current = value.length;
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <Stack spacing={1}>
        {value.map((tag, i) => (
          <Box
            key={`row-${i}-${tag.key}-${tag.value}`}
            sx={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 1 }}
          >
            <TextField
              size="small"
              label={t("project.field.tags.key")}
              value={tag.key}
              onChange={(event) => updateRow(i, { key: event.target.value })}
              inputRef={(el) => {
                keyInputRefs.current[i] = el;
              }}
              sx={{ flex: 1 }}
            />
            <TextField
              size="small"
              label={t("project.field.tags.value")}
              value={tag.value}
              onChange={(event) => updateRow(i, { value: event.target.value })}
              sx={{ flex: 1 }}
            />
            <IconButton
              aria-label={t("common.remove")}
              onClick={() => removeRow(i)}
            >
              <Close />
            </IconButton>
          </Box>
        ))}
      </Stack>
      <Box sx={{ display: "flex", justifyContent: "flex-start" }}>
        <Button
          size="small"
          startIcon={<Add />}
          onClick={appendRow}
        >
          {t("project.field.tags.add")}
        </Button>
      </Box>
    </Box>
  );
}
