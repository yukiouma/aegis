import { useState, type ChangeEvent } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Switch,
  TextField,
  Typography,
  type SelectChangeEvent,
} from "@aegis/ui/mui";
import { useI18n, type Locale } from "@aegis/ui/i18n";
import { useThemeMode } from "@aegis/ui/theme";

import { useCurrentUser } from "../../auth/data/current-user";
import { useLogout } from "../../auth/data/logout";
import { useUpdatePassword } from "../data/update-password";
import { errorMessage } from "../../../shared/api/error";

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale, t } = useI18n();
  const navigate = useNavigate();
  const currentUser = useCurrentUser();
  const updatePassword = useUpdatePassword();
  const logout = useLogout();

  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);
  const [confirmDialogOpen, setConfirmDialogOpen] = useState(false);
  const [password, setPassword] = useState("");

  const userCode = currentUser.data?.code;

  const handleThemeChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? "dark" : "light");
  };
  const handleLanguageChange = (event: SelectChangeEvent<Locale>) => {
    setLocale(event.target.value as Locale);
  };

  // Open the password dialog with a guaranteed-fresh field. Called
  // from the page button, not from any post-update path.
  function openPasswordDialog() {
    setPassword("");
    setPasswordDialogOpen(true);
  }

  // Close the password dialog for any reason — typed input must NOT
  // survive, so the field is always reset.
  function closePasswordDialog() {
    setPasswordDialogOpen(false);
    setPassword("");
  }

  // Move from the password dialog to the confirm dialog. The password
  // stays in state so the confirm step can submit it.
  function advanceToConfirm() {
    setPasswordDialogOpen(false);
    setConfirmDialogOpen(true);
  }

  // Confirm-dialog cancel: discard the password and close.
  function cancelConfirm() {
    setConfirmDialogOpen(false);
    setPassword("");
  }

  // Confirm-dialog confirm: run the credential update, then logout
  // and navigate. The confirm dialog stays open if the update fails
  // so the user can read the error.
  async function onConfirmUpdate() {
    if (userCode === undefined) return;
    try {
      await updatePassword.mutateAsync({ userCode, password });
      setConfirmDialogOpen(false);
      setPassword("");
      await logout.mutateAsync();
      await navigate({ to: "/login" });
    } catch (e) {
      // Leave the dialog open and let the Alert render the error.
      // updatePassword.error is read by the Alert below.
      void e;
    }
  }

  const themeLabel = t("settings.theme.label", {
    mode: t(mode === "dark" ? "settings.theme.dark" : "settings.theme.light"),
  });

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4" gutterBottom>
        {t("settings.heading")}
      </Typography>
      <FormControlLabel
        control={
          <Switch checked={mode === "dark"} onChange={handleThemeChange} />
        }
        label={themeLabel}
      />
      <FormControl size="small" sx={{ minWidth: 160 }}>
        <InputLabel id="language-label">
          {t("settings.language.label")}
        </InputLabel>
        <Select<Locale>
          labelId="language-label"
          value={locale}
          label={t("settings.language.label")}
          onChange={handleLanguageChange}
        >
          <MenuItem value="en">{t("language.english")}</MenuItem>
          <MenuItem value="zh-CN">{t("language.simplifiedChinese")}</MenuItem>
        </Select>
      </FormControl>

      <Box>
        <Button
          variant="outlined"
          color="warning"
          onClick={openPasswordDialog}
        >
          {t("settings.password.button")}
        </Button>
      </Box>

      <Dialog
        open={passwordDialogOpen}
        onClose={closePasswordDialog}
        aria-label={t("settings.password.dialog.title")}
      >
        <DialogTitle>{t("settings.password.dialog.title")}</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label={t("settings.password.dialog.field")}
            type="password"
            fullWidth
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={closePasswordDialog}>
            {t("settings.password.confirm.cancel")}
          </Button>
          <Button
            onClick={advanceToConfirm}
            variant="contained"
            disabled={password === ""}
          >
            {t("settings.password.dialog.next")}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog
        open={confirmDialogOpen}
        onClose={cancelConfirm}
        aria-label={t("settings.password.confirm.title")}
      >
        <DialogTitle>{t("settings.password.confirm.title")}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t("settings.password.confirm.message")}
          </DialogContentText>
          {updatePassword.isError && (
            <Alert severity="error" sx={{ mt: 2 }}>
              {t("settings.password.error.updateFailed", {
                message: errorMessage(updatePassword.error),
              })}
            </Alert>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={cancelConfirm} disabled={updatePassword.isPending}>
            {t("settings.password.confirm.cancel")}
          </Button>
          <Button
            onClick={() => void onConfirmUpdate()}
            variant="contained"
            disabled={updatePassword.isPending}
          >
            {t("settings.password.confirm.confirm")}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
