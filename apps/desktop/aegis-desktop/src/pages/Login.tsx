import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  FormControlLabel,
  Paper,
  Radio,
  RadioGroup,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useLogin, useLoginDomain } from "../data/auth";
import { errorMessage, httpCode } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

type LoginMethod = "account" | "domain";

/** Which terminal state the login attempt landed in, if any. */
type Outcome = "none" | "notFound" | "inactive" | "failed";

export function LoginPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useBootstrapLog();

  const login = useLogin();
  const loginDomain = useLoginDomain();

  const [method, setMethod] = useState<LoginMethod>("domain");
  const [accountCode, setAccountCode] = useState("");
  const [password, setPassword] = useState("");
  const [outcome, setOutcome] = useState<Outcome>("none");

  async function runLogin(attempt: () => Promise<void>) {
    push("info", "login.log.login.start");
    try {
      await attempt();
      push("success", "login.log.login.ok");
      await navigate({ to: "/" });
    } catch (e) {
      const failureCode = httpCode(e);
      if (failureCode === "not_found") {
        push("error", "login.log.login.notFound");
        setOutcome("notFound");
      } else if (failureCode === "user_inactive") {
        push("error", "login.log.login.inactive");
        setOutcome("inactive");
      } else {
        push("error", "login.log.login.failed", {
          message: errorMessage(e),
        });
        setOutcome("failed");
      }
    }
  }

  function onLogin() {
    push("info", "login.log.method.selected", {
      method: t(
        method === "account" ? "login.method.account" : "login.method.domain",
      ),
    });
    if (method === "domain") {
      void runLogin(() => loginDomain.mutateAsync());
    } else {
      void runLogin(() => login.mutateAsync({ code: accountCode, password }));
    }
  }

  function onMethodChange(next: LoginMethod) {
    // Switching the method clears any failure outcome so a stale alert
    // does not linger when the user retries with a different flow.
    setOutcome("none");
    setMethod(next);
  }

  const loginDisabled =
    login.isPending ||
    loginDomain.isPending ||
    (method === "account" && (!accountCode || !password));

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("login.title")}
        </Typography>

        <RadioGroup
          value={method}
          onChange={(event) =>
            onMethodChange(event.target.value as LoginMethod)
          }
        >
          <FormControlLabel
            value="domain"
            control={<Radio />}
            label={t("login.method.domain")}
          />
          <FormControlLabel
            value="account"
            control={<Radio />}
            label={t("login.method.account")}
          />
        </RadioGroup>

        {method === "account" && (
          <Stack spacing={2} sx={{ maxWidth: 320, mt: 1 }}>
            <TextField
              label={t("login.field.code")}
              value={accountCode}
              onChange={(event) => setAccountCode(event.target.value)}
              size="small"
            />
            <TextField
              label={t("login.field.password")}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              size="small"
            />
          </Stack>
        )}

        <Button
          variant="contained"
          onClick={onLogin}
          disabled={loginDisabled}
          sx={{ mt: 2 }}
        >
          {t("login.action.login")}
        </Button>

        {/* Outcome UI lives below the form so a failure keeps the
            Register / admin-hint affordance visible regardless of which
            method the user is on. */}
        {outcome === "notFound" && (
          <Box sx={{ mt: 2 }}>
            <Alert severity="warning" sx={{ mb: 1 }}>
              {t("login.hint.notFound")}
            </Alert>
            <Button
              variant="outlined"
              onClick={() => void navigate({ to: "/register" })}
            >
              {t("login.action.register")}
            </Button>
          </Box>
        )}

        {outcome === "inactive" && (
          <Alert severity="warning" sx={{ mt: 2 }}>
            {t("login.hint.inactive")}
          </Alert>
        )}

        <BootstrapLog entries={entries} />
      </Paper>
    </Box>
  );
}