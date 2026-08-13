import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { api, type Identity } from "../api";
import { errorMessage } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

export function RegisterPage() {
  const { t } = useI18n();
  const { entries, push } = useBootstrapLog();

  const [identity, setIdentity] = useState<Identity | null>(null);
  const [userName, setUserName] = useState("");
  const [password, setPassword] = useState("");
  const [inFlight, setInFlight] = useState(false);
  const [registered, setRegistered] = useState(false);

  // React StrictMode invokes effects twice in development. The ref keeps
  // the identity lookup to a single request.
  const lookupStarted = useRef(false);

  useEffect(() => {
    if (lookupStarted.current) return;
    lookupStarted.current = true;

    void (async () => {
      push("info", "register.log.identity.start");
      try {
        const info = await api.getDomainUserInfo();
        push("success", "register.log.identity.ok", { userid: info.userid });
        setIdentity(info);
      } catch (e) {
        push("error", "register.log.identity.failed", {
          message: errorMessage(e),
        });
      }
    })();
  }, [push]);

  async function onRegister() {
    if (!identity) return;
    setInFlight(true);
    push("info", "register.log.register.start");
    try {
      await api.registerUser({
        userCode: identity.userid,
        userName,
        domainName: identity.domain,
        hostname: identity.hostMachine,
        sid: identity.sid,
        password,
      });
      push("success", "register.log.register.ok", { userCode: identity.userid });
      setRegistered(true);
    } catch (e) {
      push("error", "register.log.register.failed", { message: errorMessage(e) });
    } finally {
      setInFlight(false);
    }
  }

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("register.title")}
        </Typography>

        {registered && (
          <Alert severity="info">{t("register.hint.contactAdmin")}</Alert>
        )}

        {identity && !registered && (
          <Stack spacing={2} sx={{ maxWidth: 360 }}>
            <TextField
              label={t("register.field.userCode")}
              value={identity.userid}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.domainName")}
              value={identity.domain}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.hostname")}
              value={identity.hostMachine}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.sid")}
              value={identity.sid}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.userName")}
              value={userName}
              onChange={(event) => setUserName(event.target.value)}
              size="small"
            />
            <TextField
              label={t("register.field.password")}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              size="small"
            />
            <Button
              variant="contained"
              disabled={inFlight || !userName || !password}
              onClick={() => void onRegister()}
            >
              {t("register.action.register")}
            </Button>
          </Stack>
        )}

        <BootstrapLog entries={entries} />
      </Paper>
    </Box>
  );
}
