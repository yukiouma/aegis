import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { ProjectFilterBar } from "../../../features/project-list/components/ProjectFilterBar";

afterEach(() => cleanup());

function renderBar(props: {
  query?: string;
  tagQuery?: string;
  involve?: boolean;
  onQueryChange?: (v: string) => void;
  onTagQueryChange?: (v: string) => void;
  onInvolveChange?: (v: boolean) => void;
} = {}) {
  const onQueryChange = props.onQueryChange ?? vi.fn();
  const onTagQueryChange = props.onTagQueryChange ?? vi.fn();
  const onInvolveChange = props.onInvolveChange ?? vi.fn();
  return {
    onQueryChange,
    onTagQueryChange,
    onInvolveChange,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectFilterBar
            query={props.query ?? ""}
            tagQuery={props.tagQuery ?? ""}
            involve={props.involve ?? false}
            onQueryChange={onQueryChange}
            onTagQueryChange={onTagQueryChange}
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

describe("ProjectFilterBar — tag filter", () => {
  it("renders the tag filter field with the current value", () => {
    renderBar({ tagQuery: "demo" });
    expect(screen.getByLabelText(/filter by tag/i)).toHaveValue("demo");
  });

  it("calls onTagQueryChange when the tag filter field changes", async () => {
    const { onTagQueryChange } = renderBar();
    await userEvent.type(screen.getByLabelText(/filter by tag/i), "x");
    expect(onTagQueryChange).toHaveBeenLastCalledWith("x");
  });

  it("leaves Involve checkbox gated as before (regression)", () => {
    renderBar({ involve: true });
    expect(screen.getByRole("checkbox", { name: /involve/i })).toBeChecked();
  });
});