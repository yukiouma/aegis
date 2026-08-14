import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { ProjectFilterBar } from "../../pages/ProjectFilterBar";

afterEach(() => cleanup());

function renderBar(props: {
  query?: string;
  involve?: boolean;
  onQueryChange?: (v: string) => void;
  onInvolveChange?: (v: boolean) => void;
} = {}) {
  const onQueryChange = props.onQueryChange ?? vi.fn();
  const onInvolveChange = props.onInvolveChange ?? vi.fn();
  return {
    onQueryChange,
    onInvolveChange,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectFilterBar
            query={props.query ?? ""}
            onQueryChange={onQueryChange}
            involve={props.involve ?? false}
            onInvolveChange={onInvolveChange}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    ),
  };
}

describe("ProjectFilterBar", () => {
  it("renders the search field with the current value", () => {
    renderBar({ query: "alpha" });
    expect(screen.getByLabelText(/search/i)).toHaveValue("alpha");
  });

  it("renders the Involve checkbox", () => {
    renderBar({ involve: false });
    expect(screen.getByRole("checkbox", { name: /involve/i })).not.toBeChecked();
  });

  it("checks the Involve checkbox when involve=true", () => {
    renderBar({ involve: true });
    expect(screen.getByRole("checkbox", { name: /involve/i })).toBeChecked();
  });

  it("calls onQueryChange when the search field changes", async () => {
    const { onQueryChange } = renderBar();
    await userEvent.type(screen.getByLabelText(/search/i), "a");
    expect(onQueryChange).toHaveBeenLastCalledWith("a");
  });

  it("calls onInvolveChange when the checkbox is clicked", async () => {
    const { onInvolveChange } = renderBar({ involve: false });
    await userEvent.click(screen.getByRole("checkbox", { name: /involve/i }));
    expect(onInvolveChange).toHaveBeenCalledWith(true);
  });
});