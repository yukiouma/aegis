import { Fragment, useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  Stack,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  Add as AddIcon,
  ExpandMore as ExpandMoreIcon,
} from "@aegis/ui/icons";
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
  onPatchState: (issueId: number, next: "opened" | "closed") => void;

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
  onPatchState,
  commentPending,
  commentError,
  onAppendComment,
  onClose,
}: Props) {
  const { t } = useI18n();

  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [newDescription, setNewDescription] = useState("");
  const [composing, setComposing] = useState(false);
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
      setComposing(false);
      setCommentDraft(null);
    }
  }, [open]);

  const handleSubmitNew = () => {
    onCreate(newDescription.trim());
    setNewDescription("");
    setComposing(false);
  };

  const handleCancelNew = () => {
    setNewDescription("");
    setComposing(false);
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        {t("crf.missionIssue.dialog.title", { scope: scopeLabel(scope) })}
      </DialogTitle>
      <DialogContent>
        {canCreate && (
          <Box sx={{ mb: 2 }}>
            {composing ? (
              <Stack spacing={1}>
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
                <Box
                  sx={{
                    display: "flex",
                    justifyContent: "flex-end",
                    gap: 1,
                  }}
                >
                  <Button
                    onClick={handleCancelNew}
                    disabled={createPending}
                  >
                    {t("crf.missionIssue.detail.cancel")}
                  </Button>
                  <Button
                    variant="contained"
                    onClick={handleSubmitNew}
                    disabled={
                      createPending || newDescription.trim() === ""
                    }
                  >
                    {t("crf.missionIssue.create.submit")}
                  </Button>
                </Box>
              </Stack>
            ) : (
              <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
                <Button
                  variant="outlined"
                  startIcon={<AddIcon />}
                  onClick={() => setComposing(true)}
                  data-testid="mission-issue-new-issue"
                >
                  {t("crf.missionIssue.create.newIssue")}
                </Button>
              </Box>
            )}
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
                      <FormControlLabel
                        sx={{ m: 0, gap: 1 }}
                        data-testid={`mission-issue-switch-${issue.id}`}
                        control={
                          <Switch
                            size="small"
                            checked={issue.state === "opened"}
                            disabled={!canActOnIssue || patchPending}
                            onChange={(_, checked) =>
                              onPatchState(
                                issue.id,
                                checked ? "opened" : "closed",
                              )
                            }
                          />
                        }
                        label={
                          <Typography
                            variant="body2"
                            color={
                              issue.state === "opened"
                                ? "warning.main"
                                : "text.secondary"
                            }
                          >
                            {t(
                              issue.state === "opened"
                                ? "crf.missionIssue.state.opened"
                                : "crf.missionIssue.state.closed",
                            )}
                          </Typography>
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
                          commentDraft={commentDraft}
                          setCommentDraft={setCommentDraft}
                          canComment={canComment}
                          commentPending={commentPending}
                          commentError={commentError}
                          resolveName={resolveName}
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
  commentDraft: { id: number; value: string } | null;
  setCommentDraft: (v: { id: number; value: string } | null) => void;
  canComment: boolean;
  commentPending: boolean;
  commentError: ApiError | null;
  resolveName: (userCode: string) => string;
  onAppendComment: (id: number, content: string) => void;
}

function IssueDetails({
  issue,
  commentDraft,
  setCommentDraft,
  canComment,
  commentPending,
  commentError,
  resolveName,
  onAppendComment,
}: IssueDetailsProps) {
  const { t } = useI18n();
  const composingComment = commentDraft?.id === issue.id;
  const draft = composingComment ? commentDraft!.value : "";

  const handleStartComment = () =>
    setCommentDraft({ id: issue.id, value: "" });
  const handleSubmitComment = () => {
    onAppendComment(issue.id, draft.trim());
    setCommentDraft(null);
  };
  const handleCancelComment = () => setCommentDraft(null);

  return (
    <Stack spacing={2} sx={{ py: 1 }}>
      {/* Description block (read-only) */}
      <Typography variant="body2">{issue.description}</Typography>

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
        <Box>
          {composingComment ? (
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
              <Box
                sx={{
                  display: "flex",
                  justifyContent: "flex-end",
                  gap: 1,
                }}
              >
                <Button
                  onClick={handleCancelComment}
                  disabled={commentPending}
                >
                  {t("crf.missionIssue.detail.cancel")}
                </Button>
                <Button
                  variant="contained"
                  disabled={commentPending || draft.trim() === ""}
                  onClick={handleSubmitComment}
                >
                  {t("crf.missionIssue.detail.submitComment")}
                </Button>
              </Box>
            </Stack>
          ) : (
            <Box sx={{ display: "flex", justifyContent: "flex-end" }}>
              <Button
                variant="contained"
                startIcon={<AddIcon />}
                onClick={handleStartComment}
                data-testid="mission-issue-comment-add"
              >
                {t("crf.missionIssue.detail.addComment")}
              </Button>
            </Box>
          )}
        </Box>
      )}
    </Stack>
  );
}