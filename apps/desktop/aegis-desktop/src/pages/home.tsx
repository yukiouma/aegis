import { useState } from "react";
import { Box, Button, Stack, TextField, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../api";

export function HomePage() {
  const { t } = useI18n();
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");
  const [loggedIn, setLoggedIn] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refreshLoginState() {
    try {
      setLoggedIn(await api.isLoggedIn());
    } catch (e) {
      setError(String(e));
    }
  }

  async function onLogin() {
    setError(null);
    try {
      await api.login(code, password);
      await refreshLoginState();
    } catch (e) {
      setError(String(e));
    }
  }

  async function onLogout() {
    setError(null);
    try {
      await api.logout();
      await refreshLoginState();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("home.heading")}
      </Typography>
      <Typography variant="body1" sx={{ mb: 3 }}>
        {t("home.welcome")}
      </Typography>

      <Stack direction="row" spacing={2} sx={{ alignItems: "center", mb: 2 }}>
        <TextField
          label="code"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          size="small"
        />
        <TextField
          label="password"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          size="small"
        />
        <Button variant="contained" onClick={onLogin}>
          Login
        </Button>
        <Button variant="outlined" onClick={onLogout}>
          Logout
        </Button>
      </Stack>

      <Typography variant="body2">
        Logged in: {loggedIn === null ? "?" : String(loggedIn)}
      </Typography>
      {error && (
        <Typography variant="body2" color="error">
          {error}
        </Typography>
      )}
    </Box>
  );
}
