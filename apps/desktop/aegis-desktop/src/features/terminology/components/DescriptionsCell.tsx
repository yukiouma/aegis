import { Box, Chip, Typography } from "@aegis/ui/mui";

export interface DescriptionsCellProps {
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

/**
 * Renders the three description fields (synonym / definition /
 * NCI preferred term) as `SYN / DEF / NCI` chip-prefixed rows.
 * Empty / whitespace fields are skipped entirely so the cell
 * collapses to zero rows when every field is blank.
 */
export function DescriptionsCell({
  synonym,
  definition,
  nciPreferredTerm,
}: DescriptionsCellProps) {
  const rows = (
    [
      ["SYN", synonym],
      ["DEF", definition],
      ["NCI", nciPreferredTerm],
    ] as Array<[string, string]>
  ).filter(([, v]) => v.trim() !== "");

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 0.5 }}>
      {rows.map(([label, value]) => (
        <Box
          key={label}
          sx={{ display: "flex", gap: 1, alignItems: "flex-start" }}
        >
          <Chip label={label} size="small" />
          <Typography variant="body2" sx={{ whiteSpace: "pre-wrap" }}>
            {value}
          </Typography>
        </Box>
      ))}
    </Box>
  );
}