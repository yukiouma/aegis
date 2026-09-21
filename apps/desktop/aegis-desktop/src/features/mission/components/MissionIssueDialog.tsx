import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { ExpandMore as ExpandMoreIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type {
  ApiError,
  IssueViewResponse,
  MissionViewResponse,
  UserView,
} from "../../../shared/api";

export type IssueScope =
  | { kind: "form" }
  | { kind: "item"; itemId: number; itemCode: string };

interface Props {
  open: boolean;
  scope: IssueScope;
  mission: MissionViewResponse;
  issues: IssueViewResponse[];
  currentUser?: UserView;
  canCreate: boolean;
  canActOnIssue: boolean;
  canComment: boolean;

  createPending: boolean;
  createError: ApiError | null;
  onCreate: (description: string) => void;

  patchPending: boolean;
  patchError: ApiError | null;
  onPatchState: (issueId: number, next: "opened" | "closed") => void;

  updateDescPending: boolean;
  updateDescError: ApiError | null;
  onUpdateDescription: (issueId: number, description: string) => void;

  commentPending: boolean;
  commentError: ApiError | null;
  onAppendComment: (issueId: number, content: string) => void;

  onClose: () => void;
}

function scopeLabel(scope: IssueScope): string {
  return scope.kind === "form" ? "Form" : `Item ${scope.itemCode}`;
}

export function MissionIssueDialog({
  open,
  scope,
  mission: _mission,
  issues,
  canCreate: _canCreate,
  canActOnIssue: _canActOnIssue,
  canComment: _canComment,
  createPending: _createPending,
  createError: _createError,
  onCreate: _onCreate,
  patchPending: _patchPending,
  patchError: _patchError,
  onPatchState: _onPatchState,
  updateDescPending: _updateDescPending,
  updateDescError: _updateDescError,
  onUpdateDescription: _onUpdateDescription,
  commentPending: _commentPending,
  commentError: _commentError,
  onAppendComment: _onAppendComment,
  onClose,
}: Props) {
  const { t } = useI18n();

  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [_newDescription, setNewDescription] = useState("");
  const [_editing, setEditing] = useState<{
    id: number;
    value: string;
  } | null>(null);
  const [_commentDraft, setCommentDraft] = useState<{
    id: number;
    value: string;
  } | null>(null);

  // Reset all internal state on close. Matches the existing dialog
  // convention (cf. AnnotationDialog). Matches the spec §"Internal state".
  useEffect(() => {
    if (!open) {
      setExpandedId(null);
      setNewDescription("");
      setEditing(null);
      setCommentDraft(null);
    }
  }, [open]);

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        {t("crf.missionIssue.dialog.title", { scope: scopeLabel(scope) })}
      </DialogTitle>
      <DialogContent>
        {issues.length === 0 ? (
          <Alert severity="info">{t("crf.missionIssue.dialog.empty")}</Alert>
        ) : (
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>{t("crf.missionIssue.table.reviewer")}</TableCell>
                <TableCell>
                  {t("crf.missionIssue.table.description")}
                </TableCell>
                <TableCell>{t("crf.missionIssue.table.state")}</TableCell>
                <TableCell />
              </TableRow>
            </TableHead>
            <TableBody>
              {issues.map((issue) => (
                <TableRow key={issue.id}>
                  <TableCell>
                    <Tooltip title={issue.issuer}>
                      <span>{issue.issuer}</span>
                    </Tooltip>
                  </TableCell>
                  <TableCell>
                    <Typography
                      variant="body2"
                      sx={{
                        display: "-webkit-box",
                        WebkitLineClamp: 2,
                        WebkitBoxOrient: "vertical",
                        overflow: "hidden",
                      }}
                    >
                      {issue.description}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip
                      size="small"
                      variant="outlined"
                      label={t(
                        issue.state === "opened"
                          ? "crf.missionIssue.state.opened"
                          : "crf.missionIssue.state.closed",
                      )}
                      color={issue.state === "opened" ? "warning" : "default"}
                      sx={
                        issue.state === "closed"
                          ? { borderStyle: "dashed" }
                          : undefined
                      }
                    />
                  </TableCell>
                  <TableCell>
                    <IconButton
                      size="small"
                      onClick={() =>
                        setExpandedId(
                          expandedId === issue.id ? null : issue.id,
                        )
                      }
                      aria-label="expand"
                    >
                      <ExpandMoreIcon
                        sx={{
                          transform:
                            expandedId === issue.id
                              ? "rotate(180deg)"
                              : "none",
                          transition: "transform 150ms",
                        }}
                      />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>{t("common.close")}</Button>
      </DialogActions>
    </Dialog>
  );
}