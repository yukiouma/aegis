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

import { useDomainUserInfo, useRegisterUser } from "../data/user";
import { errorMessage } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

export function RegisterPage() {
  const { t } = useI18n();
  const { entries, push } = useBootstrapLog();

  const identity = useDomainUserInfo();
  const register = useRegisterUser();

  const [userName, setUserName] = useState("");
  const [password, setPassword] = useState("");
  const [registered, setRegistered] = useState(false);

  // React StrictMode invokes effects twice in development. The ref
  // keeps the identity lookup to a single request.
  const lookedUp = useRef(false);

  useEffect(() => {
    if (lookedUp.current) return;
    lookedUp.current = true;

    push("info", "register.log.identity.start");
    void (async () => {
      const r = await identity.refetch();
      if (r.isError || r.data === undefined) {
        push("error", "register.log.identity.failed", {
          message: errorMessage(r.error ?? "no data"),
        });
        return;
      }
      push("success", "register.log.identity.ok", { userid: r.data.userid });
    })();
  }, [push, identity]);

  async function onRegister() {
    const info = identity.data;
    if (!info) return;
    push("info", "register.log.register.start");
    try {
      await register.mutateAsync({
        userCode: info.userid,
        userName,
        domainName: info.domain,
        hostname: info.hostMachine,
        sid: info.sid,
        password,
      });
      push("success", "register.log.register.ok", { userCode: info.userid });
      setRegistered(true);
    } catch (e) {
      push("error", "register.log.register.failed", {
        message: errorMessage(e),
      });
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

        {identity.data && !registered && (
          <Stack spacing={2} sx={{ maxWidth: 360 }}>
            <TextField
              label={t("register.field.userCode")}
              value={identity.data.userid}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.domainName")}
              value={identity.data.domain}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.hostname")}
              value={identity.data.hostMachine}
              disabled
              size="small"
            />
            <TextField
              label={t("register.field.sid")}
              value={identity.data.sid}
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
              disabled={register.isPending || !userName || !password}
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