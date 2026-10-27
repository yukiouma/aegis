import { describe, expect, it } from "vitest";
import { createTheme } from "@aegis/ui/mui";

import { pickContrastText } from "../../../features/project-workspace/components/ContrastArcLabel";

describe("pickContrastText", () => {
  const light = createTheme({ palette: { mode: "light" } });
  const dark = createTheme({ palette: { mode: "dark" } });

  it("returns white on the light theme's success green", () => {
    expect(pickContrastText(light.palette.success.main, light)).toBe("#fff");
  });

  it("returns black on the light theme's warning orange", () => {
    expect(pickContrastText(light.palette.warning.main, light)).toBe("#000");
  });

  it("keeps every theme status fill legible", () => {
    for (const theme of [light, dark]) {
      for (const fill of [
        theme.palette.success.main,
        theme.palette.warning.main,
      ]) {
        const text = pickContrastText(fill, theme);
        expect(["#fff", "#000"]).toContain(text);
      }
    }
  });
});
