import "@testing-library/jest-dom/vitest";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { DomainFilterBar } from "./DomainFilterBar";

function wrap(ui: React.ReactNode) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale="en">{ui}</AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("DomainFilterBar", () => {
  it("calls onQueryChange when the user types", async () => {
    const onChange = vi.fn();
    wrap(<DomainFilterBar query="" onQueryChange={onChange} />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "a");
    expect(onChange).toHaveBeenCalledWith("a");
  });

  it("renders the controlled value in the text field", () => {
    wrap(<DomainFilterBar query="AE" onQueryChange={() => {}} />);
    expect(screen.getByDisplayValue("AE")).toBeInTheDocument();
  });
});