import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { LanguageDropdown } from "./LanguageDropdown";

function wrap(ui: React.ReactNode) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider locale="en">{ui}</AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("LanguageDropdown", () => {
  afterEach(cleanup);

  it("renders the selected language code in the trigger", () => {
    wrap(<LanguageDropdown options={["en", "zh"]} value="en" onChange={() => {}} />);
    expect(screen.getByRole("combobox", { name: /Language/ })).toHaveTextContent(
      "en",
    );
  });

  it("calls onChange with the selected code", async () => {
    const onChange = vi.fn();
    wrap(<LanguageDropdown options={["en", "zh"]} value={null} onChange={onChange} />);
    await userEvent.click(screen.getByRole("combobox", { name: /Language/ }));
    const listbox = await screen.findByRole("listbox");
    await userEvent.click(within(listbox).getByRole("option", { name: "zh" }));
    expect(onChange).toHaveBeenCalledWith("zh");
  });

  it("is disabled when options is empty", () => {
    wrap(<LanguageDropdown options={[]} value={null} onChange={() => {}} />);
    expect(
      screen.getByRole("combobox", { name: /Language/ }),
    ).toHaveAttribute("aria-disabled", "true");
  });
});