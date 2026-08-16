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

import { useHealthz, useIsLoggedIn } from "../data/probes";
import { errorMessage } from "../../../shared/api/error";
import { BootstrapLog, useBootstrapLog } from "../../../shared/components/BootstrapLog";

export function BootstrapPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { entries, push } = useBootstrapLog();

  const [activeStep, setActiveStep] = useState(0);
  const [healthFailed, setHealthFailed] = useState(false);
  const [loginStatusFailed, setLoginStatusFailed] = useState(false);

  // Bootstrap probes are disabled by default; the page drives them
  // manually via `refetch()` so they fire exactly once on mount.
  const health = useHealthz();
  const status = useIsLoggedIn();

  // React StrictMode invokes effects twice in development. The ref
  // keeps the orchestrator to a single run.
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    void (async () => {
      push("info", "bootstrap.log.healthCheck.start");
      const h = await health.refetch();
      if (h.isError || h.data === undefined) {
        push("error", "bootstrap.log.healthCheck.failed", {
          message: errorMessage(h.error ?? "no data"),
        });
        setHealthFailed(true);
        return;
      }
      push("success", "bootstrap.log.healthCheck.ok", { status: h.data });
      setActiveStep(1);

      push("info", "bootstrap.log.loginStatus.start");
      const s = await status.refetch();
      if (s.isError || s.data === undefined) {
        push("error", "bootstrap.log.loginStatus.failed", {
          message: errorMessage(s.error ?? "no data"),
        });
        setLoginStatusFailed(true);
        return;
      }
      if (s.data) {
        push("success", "bootstrap.log.loginStatus.ok");
        await navigate({ to: "/" });
      } else {
        push("info", "bootstrap.log.loginStatus.notLoggedIn");
        await navigate({ to: "/login" });
      }
    })();
  }, [navigate, push, health, status]);

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