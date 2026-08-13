import { useCallback, useEffect, useRef, useState } from "react";
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
  Step,
  StepContent,
  StepLabel,
  Stepper,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../api";
import { errorMessage, httpCode } from "../api/error";
import { SplashLog, useSplashLog } from "../components/SplashLog";

type LoginMethod = "account" | "domain";

/** Which terminal state the login attempt landed in, if any. */
type Outcome = "none" | "notFound" | "inactive" | "failed";

export function SplashPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useSplashLog();

  const [activeStep, setActiveStep] = useState(0);
  const [healthFailed, setHealthFailed] = useState(false);
  const [method, setMethod] = useState<LoginMethod>("account");
  const [accountCode, setAccountCode] = useState("");
  const [password, setPassword] = useState("");
  const [inFlight, setInFlight] = useState(false);
  const [outcome, setOutcome] = useState<Outcome>("none");

  // React StrictMode invokes effects twice in development. The ref keeps
  // the health check to a single request.
  const healthStarted = useRef(false);

  useEffect(() => {
    if (healthStarted.current) return;
    healthStarted.current = true;

    void (async () => {
      push("info", "splash.log.healthCheck.start");
      try {
        const status = await api.healthz();
        push("success", "splash.log.healthCheck.ok", { status });
        setActiveStep(1);
      } catch (e) {
        push("error", "splash.log.healthCheck.failed", {
          message: errorMessage(e),
        });
        setHealthFailed(true);
      }
    })();
  }, [push]);

  const runLogin = useCallback(
    async (attempt: () => Promise<void>) => {
      setInFlight(true);
      setOutcome("none");
      push("info", "splash.log.login.start");
      try {
        await attempt();
        push("success", "splash.log.login.ok");
        await navigate({ to: "/" });
      } catch (e) {
        const failureCode = httpCode(e);
        if (failureCode === "not_found") {
          push("error", "splash.log.login.notFound");
          setOutcome("notFound");
        } else if (failureCode === "user_inactive") {
          push("error", "splash.log.login.inactive");
          setOutcome("inactive");
        } else {
          push("error", "splash.log.login.failed", { message: errorMessage(e) });
          setOutcome("failed");
        }
      } finally {
        setInFlight(false);
      }
    },
    [navigate, push],
  );

  function onContinue() {
    push("info", "splash.log.method.selected", {
      method: t(
        method === "account" ? "splash.method.account" : "splash.method.domain",
      ),
    });
    setActiveStep(2);
  }

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("splash.title")}
        </Typography>

        <Stepper activeStep={activeStep} orientation="vertical">
          <Step>
            <StepLabel error={healthFailed}>{t("splash.step.health")}</StepLabel>
          </Step>

          <Step>
            <StepLabel>{t("splash.step.method")}</StepLabel>
            <StepContent slotProps={{ transition: { unmountOnExit: true } }}>
              {activeStep === 1 && (
                <>
                  <RadioGroup
                    value={method}
                    onChange={(event) =>
                      setMethod(event.target.value as LoginMethod)
                    }
                  >
                    <FormControlLabel
                      value="account"
                      control={<Radio />}
                      label={t("splash.method.account")}
                    />
                    <FormControlLabel
                      value="domain"
                      control={<Radio />}
                      label={t("splash.method.domain")}
                    />
                  </RadioGroup>
                  <Button variant="contained" onClick={onContinue} sx={{ mt: 1 }}>
                    {t("splash.method.continue")}
                  </Button>
                </>
              )}
            </StepContent>
          </Step>

          <Step>
            <StepLabel error={outcome !== "none"}>
              {t("splash.step.credentials")}
            </StepLabel>
            <StepContent slotProps={{ transition: { unmountOnExit: true } }}>
              {activeStep === 2 && (
                <>
                  {method === "account" ? (
                    <Stack spacing={2} sx={{ maxWidth: 320 }}>
                      <TextField
                        label={t("splash.field.code")}
                        value={accountCode}
                        onChange={(event) => setAccountCode(event.target.value)}
                        size="small"
                      />
                      <TextField
                        label={t("splash.field.password")}
                        type="password"
                        value={password}
                        onChange={(event) => setPassword(event.target.value)}
                        size="small"
                      />
                      <Button
                        variant="contained"
                        disabled={inFlight || !accountCode || !password}
                        onClick={() =>
                          void runLogin(() => api.login(accountCode, password))
                        }
                      >
                        {t("splash.action.login")}
                      </Button>
                    </Stack>
                  ) : (
                    <Button
                      variant="contained"
                      disabled={inFlight}
                      onClick={() => void runLogin(() => api.loginDomain())}
                    >
                      {t("splash.action.loginDomain")}
                    </Button>
                  )}

                  {outcome === "notFound" && (
                    <Box sx={{ mt: 2 }}>
                      <Alert severity="warning" sx={{ mb: 1 }}>
                        {t("splash.hint.notFound")}
                      </Alert>
                      <Button
                        variant="outlined"
                        onClick={() => void navigate({ to: "/register" })}
                      >
                        {t("splash.action.register")}
                      </Button>
                    </Box>
                  )}

                  {outcome === "inactive" && (
                    <Alert severity="warning" sx={{ mt: 2 }}>
                      {t("splash.hint.inactive")}
                    </Alert>
                  )}
                </>
              )}
            </StepContent>
          </Step>
        </Stepper>

        <SplashLog entries={entries} />
      </Paper>
    </Box>
  );
}
