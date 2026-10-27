import { getContrastRatio, useTheme, type Theme } from "@aegis/ui/mui";
import { PieArcLabel, type PieArcLabelProps } from "@aegis/ui/charts";

/**
 * Pick the text colour with the higher WCAG contrast against
 * `background`, choosing between pure white and pure black.
 *
 * `theme.palette.getContrastText` is deliberately not used here: it
 * compares against `palette.text.primary`, which is 87% opaque black,
 * so the alpha is dropped when the ratio is computed and the colour it
 * returns can fall below 3:1 once it is actually composited over the
 * fill. Pure black and pure white avoid that.
 */
export function pickContrastText(background: string, theme: Theme): string {
  const { white, black } = theme.palette.common;
  return getContrastRatio(background, white) >=
    getContrastRatio(background, black)
    ? white
    : black;
}

/**
 * A `pieArcLabel` slot that colours the label to contrast with the arc
 * it sits on.
 *
 * The library's default label colour is `palette.text.primary`, which
 * is unreadable on a saturated fill — on MUI's `success.main` green it
 * lands at roughly 3:1. Every arc in this chart carries a percentage,
 * so the label has to be legible against its own fill.
 */
export function ContrastArcLabel(props: PieArcLabelProps) {
  const theme = useTheme();
  const { color, ...rest } = props;
  return (
    <PieArcLabel
      {...rest}
      color={color}
      style={{ fill: pickContrastText(color, theme) }}
    />
  );
}
