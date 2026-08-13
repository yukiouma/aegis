import { useEffect, useRef, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Box,
  Paper,
  Step,
  StepLabel,
  Stepper,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../api";
import { errorMessage } from "../api/error";
import { BootstrapLog, useBootstrapLog } from "../components/BootstrapLog";

export function BootstrapPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useBootstrapLog();

  const [activeStep, setActiveStep] = useState(0);
  const [healthFailed, setHealthFailed] = useState(false);
  const [loginStatusFailed, setLoginStatusFailed] = useState(false);

  // React StrictMode invokes effects twice in development. The ref keeps
  // the health check and login-status probe to a single request each.
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    void (async () => {
      push("info", "bootstrap.log.healthCheck.start");
      try {
        const status = await api.healthz();
        push("success", "bootstrap.log.healthCheck.ok", { status });
        setActiveStep(1);
      } catch (e) {
        push("error", "bootstrap.log.healthCheck.failed", { message: errorMessage(e) });
        setHealthFailed(true);
        return;
      }

      push("info", "bootstrap.log.loginStatus.start");
      let loggedIn = false;
      try {
        loggedIn = await api.isLoggedIn();
      } catch (e) {
        push("error", "bootstrap.log.loginStatus.failed", { message: errorMessage(e) });
        setLoginStatusFailed(true);
        return;
      }
      if (loggedIn) {
        push("success", "bootstrap.log.loginStatus.ok");
        await navigate({ to: "/" });
      } else {
        push("info", "bootstrap.log.loginStatus.notLoggedIn");
        await navigate({ to: "/login" });
      }
    })();
  }, [navigate, push]);

  return (
    <Box sx={{ display: "flex", justifyContent: "center", p: 4 }}>
      <Paper sx={{ p: 4, width: 560, maxWidth: "100%" }}>
        <Typography variant="h4" gutterBottom>
          {t("bootstrap.title")}
        </Typography>

        <Stepper activeStep={activeStep} orientation="vertical">
          <Step>
            <StepLabel error={healthFailed}>
              {t("bootstrap.step.health")}
            </StepLabel>
          </Step>
          <Step>
            <StepLabel error={loginStatusFailed}>
              {t("bootstrap.step.loginStatus")}
            </StepLabel>
          </Step>
        </Stepper>

        <BootstrapLog entries={entries} />
      </Paper>
    </Box>
  );
}
