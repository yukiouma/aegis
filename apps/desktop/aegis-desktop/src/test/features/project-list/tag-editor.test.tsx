import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { TagEditor } from "../../../features/project-list/components/TagEditor";
import type { Tag } from "../../../shared/api";

const tagProduct: Tag = { key: "Product", value: "DEMO-001" };
const tagClient: Tag = { key: "Client", value: "ACME" };

afterEach(() => cleanup());

function renderEditor(props: {
  value?: Tag[];
  onChange?: (next: Tag[]) => void;
  onTouched?: () => void;
} = {}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <TagEditor
          value={props.value ?? []}
          onChange={props.onChange ?? vi.fn()}
          onTouched={props.onTouched}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("TagEditor", () => {
  it("renders no rows when value is empty and an Add tag button", () => {
    renderEditor();
    // No rows means no "Tag key" labels exist (the Add button is not labeled "Tag key").
    expect(screen.queryAllByLabelText(/tag key/i)).toHaveLength(0);
    expect(screen.getByRole("button", { name: /add tag/i })).toBeInTheDocument();
  });

  it("renders one row per value entry, each with a key and value TextField", () => {
    renderEditor({ value: [tagProduct, tagClient] });
    const keys = screen.getAllByLabelText(/tag key/i);
    const values = screen.getAllByLabelText(/tag value/i);
    expect(keys).toHaveLength(2);
    expect(values).toHaveLength(2);
    expect(keys[0]).toHaveValue("Product");
    expect(values[0]).toHaveValue("DEMO-001");
    expect(keys[1]).toHaveValue("Client");
    expect(values[1]).toHaveValue("ACME");
  });

  it("clicking Add tag appends an empty row and fires onChange", async () => {
    const onChange = vi.fn();
    renderEditor({ value: [tagProduct], onChange });
    await userEvent.click(screen.getByRole("button", { name: /add tag/i }));
    expect(onChange).toHaveBeenCalledTimes(1);
    const next = onChange.mock.calls[0][0] as Tag[];
    expect(next).toEqual([tagProduct, { key: "", value: "" }]);
  });

  it("clicking a row's remove button drops that row and fires onChange", async () => {
    const onChange = vi.fn();
    renderEditor({ value: [tagProduct, tagClient], onChange });
    // Both rows have a remove button; click the first one.
    const removes = screen.getAllByRole("button", { name: /remove/i });
    await userEvent.click(removes[0]);
    expect(onChange).toHaveBeenCalledTimes(1);
    const next = onChange.mock.calls[0][0] as Tag[];
    expect(next).toEqual([tagClient]);
  });

  it("editing a key updates only that key in the row and fires onChange", async () => {
    const onChange = vi.fn();
    renderEditor({ value: [tagProduct], onChange });
    const keyInput = screen.getByDisplayValue("Product");
    // userEvent.clear is unreliable against MUI's controlled TextField
    // under jsdom (the controlled value re-syncs on focus). Drive the
    // change directly so we exercise the same handler the user would.
    fireEvent.change(keyInput, { target: { value: "Owner" } });
    expect(onChange).toHaveBeenCalled();
    const lastCall = onChange.mock.calls.at(-1)?.[0] as Tag[] | undefined;
    expect(lastCall).toBeDefined();
    expect(lastCall![0].key).toBe("Owner");
    expect(lastCall![0].value).toBe("DEMO-001");
  });

  it("fires onTouched on first interaction only", async () => {
    const onTouched = vi.fn();
    renderEditor({ value: [tagProduct], onTouched });
    const removes = screen.getAllByRole("button", { name: /remove/i });
    await userEvent.click(removes[0]);
    await userEvent.click(screen.getByRole("button", { name: /add tag/i }));
    expect(onTouched).toHaveBeenCalledTimes(1);
  });
});
