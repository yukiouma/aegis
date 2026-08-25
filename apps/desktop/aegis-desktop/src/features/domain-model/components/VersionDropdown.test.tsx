import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import type { SdtmVersionView } from "../../../shared/api";
import { VersionDropdown } from "./VersionDropdown";

function wrap(ui: React.ReactNode) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale="en">{ui}</AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

const versions: SdtmVersionView[] = [
  { id: 1, name: "v1", createdAt: "", updatedAt: "" },
  { id: 2, name: "v2", createdAt: "", updatedAt: "" },
];

describe("VersionDropdown", () => {
  afterEach(cleanup);

  it("renders every version's name in the trigger label when selected", () => {
    wrap(<VersionDropdown versions={versions} value={1} onChange={() => {}} />);
    expect(screen.getByRole("combobox", { name: /Version/ })).toHaveTextContent(
      "v1",
    );
  });

  it("calls onChange with the selected id", async () => {
    const onChange = vi.fn();
    wrap(<VersionDropdown versions={versions} value={null} onChange={onChange} />);
    await userEvent.click(screen.getByRole("combobox", { name: /Version/ }));
    const listbox = await screen.findByRole("listbox");
    await userEvent.click(within(listbox).getByRole("option", { name: "v2" }));
    expect(onChange).toHaveBeenCalledWith(2);
  });

  it("is disabled when versions is empty", () => {
    wrap(<VersionDropdown versions={[]} value={null} onChange={() => {}} />);
    expect(
      screen.getByRole("combobox", { name: /No versions/ }),
    ).toHaveAttribute("aria-disabled", "true");
  });
});