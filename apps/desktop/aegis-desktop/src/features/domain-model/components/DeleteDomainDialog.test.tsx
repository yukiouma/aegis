import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import type { SdtmDomainView } from "../../../shared/api";
import { DeleteDomainDialog } from "./DeleteDomainDialog";

function wrap(ui: React.ReactNode) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale="en">{ui}</AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

const row: SdtmDomainView = {
  id: 7,
  versionId: 1,
  name: "AE",
  category: "Events",
  descriptions: [],
  createdAt: "",
  updatedAt: "",
};

describe("DeleteDomainDialog", () => {
  afterEach(cleanup);

  it("renders nothing when open is false", () => {
    wrap(
      <DeleteDomainDialog
        open={false}
        row={row}
        onClose={() => {}}
        onConfirm={() => {}}
        pending={false}
        error={null}
      />,
    );
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("calls onConfirm with the row when Confirm is clicked", async () => {
    const onConfirm = vi.fn();
    wrap(
      <DeleteDomainDialog
        open={true}
        row={row}
        onClose={() => {}}
        onConfirm={onConfirm}
        pending={false}
        error={null}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));
    expect(onConfirm).toHaveBeenCalledWith(row);
  });

  it("calls onClose when Cancel is clicked", async () => {
    const onClose = vi.fn();
    wrap(
      <DeleteDomainDialog
        open={true}
        row={row}
        onClose={onClose}
        onConfirm={() => {}}
        pending={false}
        error={null}
      />,
    );
    await userEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalled();
  });

  it("disables the Confirm button while pending", () => {
    wrap(
      <DeleteDomainDialog
        open={true}
        row={row}
        onClose={() => {}}
        onConfirm={() => {}}
        pending={true}
        error={null}
      />,
    );
    expect(screen.getByRole("button", { name: /confirm/i })).toBeDisabled();
  });
});