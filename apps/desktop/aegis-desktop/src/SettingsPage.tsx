import type { ChangeEvent } from "react";
import { Box, FormControlLabel, Switch, Typography } from "@aegis/ui/mui";
import { useThemeMode } from "@aegis/ui/theme";

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();

  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? "dark" : "light");
  };

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Typography variant="h4" gutterBottom>Settings</Typography>
      <FormControlLabel
        control={<Switch checked={mode === "dark"} onChange={handleChange} />}
        label={`Theme: ${mode}`}
      />
    </Box>
  );
}
