import { Fragment, useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TextField,
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
import { errorMessage } from "../../../shared/api/error";

export type IssueScope =
  | { kind: "form" }
  | { kind: "item"; itemId: number; itemCode: string };

interface Props {
  open: boolean;
  scope: IssueScope;
  mission: MissionViewResponse;
  issues: IssueViewResponse[];
  currentUser?: UserView;
  /**
   * Resolve a `user_code` to its display `name`. Falls back to the
   * code itself when the user isn't in the cache (list not loaded
   * yet, or the user was deactivated after the comment was
   * written). Wired from `useUserNameMap()` on the page.
   */
  resolveName: (userCode: string) => string;
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
  resolveName,
  canCreate,
  canActOnIssue,
  canComment,
  createPending,
  createError,
  onCreate,
  patchPending,
  patchError,
  onPatchState,
  updateDescPending,
  updateDescError,
  onUpdateDescription,
  commentPending,
  commentError,
  onAppendComment,
  onClose,
}: Props) {
  const { t } = useI18n();

  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [newDescription, setNewDescription] = useState("");
  const [editing, setEditing] = useState<{
    id: number;
    value: string;
  } | null>(null);
  const [commentDraft, setCommentDraft] = useState<{
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
        {canCreate && (
          <Box
            sx={{
              display: "flex",
              flexDirection: "column",
              gap: 1,
              mb: 2,
            }}
          >
            <TextField
              multiline
              minRows={2}
              label={t("crf.missionIssue.create.field.description")}
              value={newDescription}
              onChange={(e) => setNewDescription(e.target.value)}
              disabled={createPending}
              data-testid="mission-issue-new-description"
            />
            {createError && (
              <Alert severity="error">{errorMessage(createError)}</Alert>
            )}
            <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
              <Button
                variant="contained"
                onClick={() => onCreate(newDescription.trim())}
                disabled={createPending || newDescription.trim() === ""}
              >
                {t("crf.missionIssue.create.submit")}
              </Button>
            </Box>
          </Box>
        )}
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
                <Fragment key={issue.id}>
                  <TableRow>
                    <TableCell>
                      <Tooltip title={issue.issuer}>
                        <span>{resolveName(issue.issuer)}</span>
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
                        color={
                          issue.state === "opened" ? "warning" : "default"
                        }
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
                  {expandedId === issue.id && (
                    <TableRow>
                      <TableCell colSpan={4} sx={{ bgcolor: "action.hover" }}>
                        <IssueDetails
                          issue={issue}
                          editing={editing}
                          setEditing={setEditing}
                          commentDraft={commentDraft}
                          setCommentDraft={setCommentDraft}
                          canActOnIssue={canActOnIssue}
                          canComment={canComment}
                          patchPending={patchPending}
                          patchError={patchError}
                          updateDescPending={updateDescPending}
                          updateDescError={updateDescError}
                          commentPending={commentPending}
                          commentError={commentError}
                          resolveName={resolveName}
                          onPatchState={onPatchState}
                          onUpdateDescription={onUpdateDescription}
                          onAppendComment={onAppendComment}
                        />
                      </TableCell>
                    </TableRow>
                  )}
                </Fragment>
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

interface IssueDetailsProps {
  issue: IssueViewResponse;
  editing: { id: number; value: string } | null;
  setEditing: (v: { id: number; value: string } | null) => void;
  commentDraft: { id: number; value: string } | null;
  setCommentDraft: (v: { id: number; value: string } | null) => void;
  canActOnIssue: boolean;
  canComment: boolean;
  patchPending: boolean;
  patchError: ApiError | null;
  updateDescPending: boolean;
  updateDescError: ApiError | null;
  commentPending: boolean;
  commentError: ApiError | null;
  resolveName: (userCode: string) => string;
  onPatchState: (id: number, next: "opened" | "closed") => void;
  onUpdateDescription: (id: number, description: string) => void;
  onAppendComment: (id: number, content: string) => void;
}

function IssueDetails({
  issue,
  editing,
  setEditing,
  commentDraft,
  setCommentDraft,
  canActOnIssue,
  canComment,
  patchPending,
  patchError,
  updateDescPending,
  updateDescError,
  commentPending,
  commentError,
  resolveName,
  onPatchState,
  onUpdateDescription,
  onAppendComment,
}: IssueDetailsProps) {
  const { t } = useI18n();
  const isEditing = editing?.id === issue.id;
  const draft =
    commentDraft?.id === issue.id ? commentDraft.value : "";

  return (
    <Stack spacing={2} sx={{ py: 1 }}>
      {/* Description block */}
      <Box>
        {isEditing ? (
          <Stack spacing={1}>
            <TextField
              multiline
              minRows={2}
              value={editing!.value}
              onChange={(e) =>
                setEditing({ id: issue.id, value: e.target.value })
              }
              disabled={updateDescPending}
              fullWidth
            />
            {updateDescError && (
              <Alert severity="error">{errorMessage(updateDescError)}</Alert>
            )}
            <Box
              sx={{
                display: "flex",
                flexDirection: "row",
                justifyContent: "flex-end",
                gap: 1,
              }}
            >
              <Button
                onClick={() => setEditing(null)}
                disabled={updateDescPending}
              >
                {t("crf.missionIssue.detail.cancel")}
              </Button>
              <Button
                variant="contained"
                onClick={() =>
                  onUpdateDescription(issue.id, editing!.value.trim())
                }
                disabled={
                  updateDescPending ||
                  editing!.value.trim() === "" ||
                  editing!.value.trim() === issue.description
                }
              >
                {t("crf.missionIssue.detail.save")}
              </Button>
            </Box>
          </Stack>
        ) : (
          <Box
            sx={{
              display: "flex",
              flexDirection: "row",
              alignItems: "flex-start",
              justifyContent: "space-between",
              gap: 1,
            }}
          >
            <Typography variant="body2" sx={{ flexGrow: 1 }}>
              {issue.description}
            </Typography>
            {canActOnIssue && (
              <Button
                size="small"
                onClick={() =>
                  setEditing({ id: issue.id, value: issue.description })
                }
              >
                {t("crf.missionIssue.detail.editDescription")}
              </Button>
            )}
          </Box>
        )}
      </Box>

      {/* State-flip button */}
      {canActOnIssue && (
        <Box>
          {patchError && (
            <Alert severity="error" sx={{ mb: 1 }}>
              {errorMessage(patchError)}
            </Alert>
          )}
          <Button
            size="small"
            variant="outlined"
            color={issue.state === "opened" ? "warning" : "primary"}
            disabled={patchPending}
            onClick={() =>
              onPatchState(
                issue.id,
                issue.state === "opened" ? "closed" : "opened",
              )
            }
            data-testid={`mission-issue-${issue.state === "opened" ? "close" : "reopen"}-${issue.id}`}
          >
            {t(
              issue.state === "opened"
                ? "crf.missionIssue.detail.close"
                : "crf.missionIssue.detail.reopen",
            )}
          </Button>
        </Box>
      )}

      {/* Comments */}
      <Box>
        <Typography variant="subtitle2" sx={{ mb: 1 }}>
          {t("crf.missionIssue.detail.comments")}
        </Typography>
        {issue.comments.length === 0 ? (
          <Alert severity="info">
            {t("crf.missionIssue.detail.noComments")}
          </Alert>
        ) : (
          <Stack spacing={1}>
            {issue.comments.map((c, i) => (
              <Box
                key={i}
                sx={{ borderLeft: 2, pl: 1, borderColor: "divider" }}
              >
                <Typography variant="body2">
                  <Tooltip title={c.user}>
                    <strong>{resolveName(c.user)}</strong>
                  </Tooltip>
                  : {c.content}
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {c.createdAt}
                </Typography>
              </Box>
            ))}
          </Stack>
        )}
      </Box>

      {/* Comment form */}
      {canComment && (
        <Stack spacing={1}>
          <TextField
            multiline
            minRows={1}
            placeholder={t("crf.missionIssue.detail.commentPlaceholder")}
            value={draft}
            onChange={(e) =>
              setCommentDraft({ id: issue.id, value: e.target.value })
            }
            disabled={commentPending}
            fullWidth
          />
          {commentError && (
            <Alert severity="error">{errorMessage(commentError)}</Alert>
          )}
          <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
            <Button
              variant="contained"
              disabled={commentPending || draft.trim() === ""}
              onClick={() => onAppendComment(issue.id, draft.trim())}
            >
              {t("crf.missionIssue.detail.addComment")}
            </Button>
          </Box>
        </Stack>
      )}
    </Stack>
  );
}