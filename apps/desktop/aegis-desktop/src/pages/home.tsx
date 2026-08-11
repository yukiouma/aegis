import { useState } from "react";
import { Box, Button, Stack, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { invoke } from "@tauri-apps/api/core";

export function HomePage() {
  const { t } = useI18n();
  const [greetMsg, setGreetMsg] = useState("");

  async function testGreet() {
    setGreetMsg(await invoke<string>("greet", { name: "Aegis" }));
  }

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("home.heading")}
      </Typography>
      <Typography variant="body1" sx={{ mb: 3 }}>
        {t("home.welcome")}
      </Typography>
      <Stack direction="row" spacing={2} sx={{ alignItems: "center" }}>
        <Button variant="contained" onClick={testGreet}>
          {t("home.testGreet")}
        </Button>
        {greetMsg && <Typography variant="body2">{greetMsg}</Typography>}
      </Stack>
    </Box>
  );
}
