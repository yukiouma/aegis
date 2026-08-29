import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { CrfItemRow } from "../../../features/crf/components/CrfItemRow";
import type { CrfItemDetail } from "../../../shared/api";

afterEach(() => cleanup());

const baseItemDetail: CrfItemDetail = {
  item: {
    id: 21,
    formId: 11,
    code: "AETERM",
    name: "Term",
    kind: "text",
    order: 0,
    notSubmitted: false,
    createdAt: "",
    updatedAt: "",
  },
  options: [
    {
      option: {
        id: 31,
        itemId: 21,
        value: "YES",
        notSubmitted: false,
        createdAt: "",
        updatedAt: "",
      },
      annotations: [],
    },
  ],
  units: [
    {
      unit: {
        id: 41,
        itemId: 21,
        value: "mg",
        notSubmitted: false,
        createdAt: "",
        updatedAt: "",
      },
      annotations: [],
    },
  ],
  annotations: [],
};

function renderRow(overrides: Partial<React.ComponentProps<typeof CrfItemRow>> = {}) {
  const onCreateAnnotation = vi.fn();
  const onEditAnnotation = vi.fn();
  const onDeleteAnnotation = vi.fn();
  const onClearNotSubmitted = vi.fn();
  const utils = render(
    <AegisI18nProvider>
      <CrfItemRow
        itemDetail={baseItemDetail}
        colorByDomainAnnotationId={new Map()}
        onCreateAnnotation={onCreateAnnotation}
        onEditAnnotation={onEditAnnotation}
        onDeleteAnnotation={onDeleteAnnotation}
        onClearNotSubmitted={onClearNotSubmitted}
        formNotSubmitted={false}
        itemNotSubmitted={false}
        noDomainAnnotations={false}
        {...overrides}
      />
    </AegisI18nProvider>,
  );
  return {
    onCreateAnnotation,
    onEditAnnotation,
    onDeleteAnnotation,
    onClearNotSubmitted,
    ...utils,
  };
}

describe("CrfItemRow", () => {
  // Existing baseline — the create-annotation entry points fire as
  // expected while the form is submitted.
  describe("when formNotSubmitted=false", () => {
    it("opens the create-annotation dialog when the item name is clicked", () => {
      const { onCreateAnnotation } = renderRow();
      fireEvent.click(screen.getByTestId("crf-item-name-21"));
      expect(onCreateAnnotation).toHaveBeenCalledWith({
        kind: "item",
        id: 21,
      });
    });

    it("opens the create-annotation dialog when the option value is clicked", () => {
      const { onCreateAnnotation } = renderRow();
      fireEvent.click(screen.getByTestId("crf-option-31"));
      expect(onCreateAnnotation).toHaveBeenCalledWith({
        kind: "option",
        id: 31,
      });
    });

    it("opens the create-annotation dialog when the unit value is clicked", () => {
      const { onCreateAnnotation } = renderRow();
      fireEvent.click(screen.getByTestId("crf-unit-41"));
      expect(onCreateAnnotation).toHaveBeenCalledWith({
        kind: "unit",
        id: 41,
      });
    });

    it("keeps the pointer cursor on the clickable labels", () => {
      renderRow();
      expect(screen.getByTestId("crf-item-name-21")).toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-option-31")).toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-unit-41")).toHaveStyle({
        cursor: "pointer",
      });
    });
  });

  // The new behavior — clicking the create entry points is a no-op
  // while the owning form is marked not-submitted, because the
  // cascade has already wiped every annotation on the form. The
  // cursor / hover affordances are dropped so the row doesn't
  // advertise a click that won't fire.
  describe("when formNotSubmitted=true", () => {
    it("ignores clicks on the item name", () => {
      const { onCreateAnnotation } = renderRow({ formNotSubmitted: true });
      fireEvent.click(screen.getByTestId("crf-item-name-21"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("ignores clicks on the option value", () => {
      const { onCreateAnnotation } = renderRow({ formNotSubmitted: true });
      fireEvent.click(screen.getByTestId("crf-option-31"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("ignores clicks on the unit value", () => {
      const { onCreateAnnotation } = renderRow({ formNotSubmitted: true });
      fireEvent.click(screen.getByTestId("crf-unit-41"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("drops the pointer cursor on the clickable labels", () => {
      renderRow({ formNotSubmitted: true });
      expect(screen.getByTestId("crf-item-name-21")).not.toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-option-31")).not.toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-unit-41")).not.toHaveStyle({
        cursor: "pointer",
      });
    });

    it("still wires the close-on-chip click for owner-level clear", () => {
      // The per-row [NOT SUBMITTED] chip on the item / option / unit
      // is a clear-flag affordance, not a create-annotation entry.
      // It must keep working even when the form is not-submitted, so
      // the user can flip the owner back to submitted without
      // touching the form-level chip.
      const { onClearNotSubmitted } = renderRow({
        itemDetail: {
          ...baseItemDetail,
          item: { ...baseItemDetail.item, notSubmitted: true },
        },
        formNotSubmitted: true,
      });
      const chip = screen.getByTestId("not-submitted-chip");
      const deleteIcon = chip
        .closest(".MuiChip-root")!
        .querySelector(".MuiChip-deleteIcon");
      expect(deleteIcon).not.toBeNull();
      fireEvent.click(deleteIcon!);
      expect(onClearNotSubmitted).toHaveBeenCalledWith({
        kind: "item",
        id: 21,
      });
    });
  });

  // When the item itself is marked not-submitted, the item-level
  // cascade has wiped the item's annotations AND the annotations
  // on its options and units, so every create-annotation entry
  // point on the row must short-circuit — not just the item.
  describe("when itemNotSubmitted=true", () => {
    it("ignores clicks on the item name", () => {
      const { onCreateAnnotation } = renderRow({ itemNotSubmitted: true });
      fireEvent.click(screen.getByTestId("crf-item-name-21"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("ignores clicks on the option value (cascade wiped option annotations)", () => {
      const { onCreateAnnotation } = renderRow({ itemNotSubmitted: true });
      fireEvent.click(screen.getByTestId("crf-option-31"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("ignores clicks on the unit value (cascade wiped unit annotations)", () => {
      const { onCreateAnnotation } = renderRow({ itemNotSubmitted: true });
      fireEvent.click(screen.getByTestId("crf-unit-41"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("drops the pointer cursor on the clickable labels", () => {
      renderRow({ itemNotSubmitted: true });
      expect(screen.getByTestId("crf-item-name-21")).not.toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-option-31")).not.toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-unit-41")).not.toHaveStyle({
        cursor: "pointer",
      });
    });
  });

  // An annotation needs a domain annotation to belong to. When the
  // form has none, every create-annotation entry point on the row
  // must short-circuit — the page also gates the form-level
  // `New annotation` menu item on the same condition.
  describe("when noDomainAnnotations=true", () => {
    it("ignores clicks on the item name", () => {
      const { onCreateAnnotation } = renderRow({
        noDomainAnnotations: true,
      });
      fireEvent.click(screen.getByTestId("crf-item-name-21"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("ignores clicks on the option value", () => {
      const { onCreateAnnotation } = renderRow({
        noDomainAnnotations: true,
      });
      fireEvent.click(screen.getByTestId("crf-option-31"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("ignores clicks on the unit value", () => {
      const { onCreateAnnotation } = renderRow({
        noDomainAnnotations: true,
      });
      fireEvent.click(screen.getByTestId("crf-unit-41"));
      expect(onCreateAnnotation).not.toHaveBeenCalled();
    });

    it("drops the pointer cursor on the clickable labels", () => {
      renderRow({ noDomainAnnotations: true });
      expect(screen.getByTestId("crf-item-name-21")).not.toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-option-31")).not.toHaveStyle({
        cursor: "pointer",
      });
      expect(screen.getByTestId("crf-unit-41")).not.toHaveStyle({
        cursor: "pointer",
      });
    });
  });
});
