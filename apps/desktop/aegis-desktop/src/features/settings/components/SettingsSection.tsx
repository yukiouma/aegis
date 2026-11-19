import type { ReactNode } from "react";
import { Card, CardContent, CardHeader, Typography } from "@aegis/ui/mui";

export interface SettingsSectionProps {
  /**
   * A stable id for the section. It labels the section for assistive
   * technology, so it has to be unique on the page.
   */
  id: string;
  /** The heading of the section. */
  title: string;
  children: ReactNode;
}

/**
 * A titled settings section.
 *
 * The card is a real `<section>` named by its own heading, so everything
 * inside it is one thing to find — for a reader using assistive
 * technology, and for the tests that assert what belongs to it.
 */
export function SettingsSection({
  id,
  title,
  children,
}: SettingsSectionProps) {
  return (
    <Card component="section" aria-labelledby={id} >
      <CardHeader
        title={
          <Typography id={id} variant="h6">
            {title}
          </Typography>
        }
      />
      <CardContent>{children}</CardContent>
    </Card>
  );
}
