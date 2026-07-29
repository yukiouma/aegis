import { useState } from "react";
import { Box, Button, Stack, Typography } from "@aegis/ui/mui";
import { invoke } from "@tauri-apps/api/core";

export function HomePage() {
  const [greetMsg, setGreetMsg] = useState("");

  async function testGreet() {
    setGreetMsg(await invoke<string>("greet", { name: "Aegis" }));
  }

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>Home</Typography>
      <Typography variant="body1" sx={{ mb: 3 }}>
        Welcome to Aegis.
      </Typography>
      <Stack direction="row" spacing={2} sx={{ alignItems: "center" }}>
        <Button variant="contained" onClick={testGreet}>
          Test greet
        </Button>
        {greetMsg && <Typography variant="body2">{greetMsg}</Typography>}
      </Stack>
    </Box>
  );
}
