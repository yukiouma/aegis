import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";

function renderPage(initialEntries: string[]) {
  return renderWithFullRouter({
    initialEntries,
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

const fakeUser = {
  id: 1,
  code: "u",
  name: "U",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const fakeForm = {
  id: 11,
  versionId: 7,
  code: "AE",
  name: "Adverse Events",
  order: 0,
  notSubmitted: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

/**
 * Composed-detail payload the mock backend returns from
 * `get_crf_form_details`. Kept minimal but exercises every CrfItemRow
 * branch: one item with a unit, an option, a form-level annotation,
 * and one item-level annotation that links back to the
 * `domainAnnotations[0]` entry — so we can assert the chip colour
 * cycle (`info` for index 0) at the same time.
 */
const fakeDetail = {
  form: fakeForm,
  formAnnotations: [
    {
      id: 100,
      domainAnnotationId: 50,
      content: "form-level note",
      assign: false,
      owner: { kind: "form", id: 11 },
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-02T00:00:00Z",
    },
  ],
  items: [
    {
      item: {
        id: 21,
        formId: 11,
        code: "AETERM",
        name: "Term",
        kind: "text",
        order: 0,
        notSubmitted: false,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-02T00:00:00Z",
      },
      options: [
        {
          option: {
            id: 31,
            itemId: 21,
            value: "YES",
            notSubmitted: false,
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-02T00:00:00Z",
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
            createdAt: "2026-01-01T00:00:00Z",
            updatedAt: "2026-01-02T00:00:00Z",
          },
          annotations: [],
        },
      ],
      annotations: [
        {
          id: 110,
          domainAnnotationId: 50,
          content: "item-level note",
          assign: true,
          owner: { kind: "item", id: 21 },
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
      ],
    },
  ],
  domainAnnotations: [
    {
      id: 50,
      formId: 11,
      name: "AE",
      description: "Adverse Events",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-02T00:00:00Z",
    },
  ],
};

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
});

describe("CrfDetailPage", () => {
  it("renders the header, the form annotation chip, and the domain annotation chip", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
    });

    renderPage(["/project/abc/crf/11"]);

    // Header: code chip + form name
    expect(await screen.findByText("Adverse Events")).toBeInTheDocument();
    expect(screen.getByText("AE")).toBeInTheDocument();

    // Domain annotation chip (renders label `AE (Adverse Events)`).
    // The header chip cycles through the same colour palette as the
    // annotation chips below — `annotationColor(0) === "info"`, which
    // MUI renders with the `MuiChip-colorInfo` class.
    const domainChip = await screen.findByTestId(
      "domain-annotation-chip-50",
    );
    expect(domainChip).toBeInTheDocument();
    expect(domainChip).toHaveClass("MuiChip-colorInfo");

    // Form-level annotation chip + item-level annotation chip
    expect(await screen.findByText("form-level note")).toBeInTheDocument();
    expect(screen.getByText("item-level note")).toBeInTheDocument();

    // Item name and the unit / option rows are present
    expect(screen.getByTestId("crf-item-name-21")).toBeInTheDocument();
    expect(screen.getByTestId("crf-unit-41")).toBeInTheDocument();
    expect(screen.getByTestId("crf-option-31")).toBeInTheDocument();
  });

  it("opens the new-annotation drawer from the hover menu over the form name", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
    });

    renderPage(["/project/abc/crf/11"]);

    const formName = await screen.findByTestId("crf-form-name");
    // Click the form name to open the action menu
    fireEvent.click(formName);
    const newAnnotationMenu = await screen.findByRole("menuitem", {
      name: /New annotation/i,
    });
    fireEvent.click(newAnnotationMenu);

    // Drawer with the Create title appears; Domain-annotation Select is
    // enabled here (vs. disabled in edit mode), so we assert it is
    // editable rather than aria-disabled.
    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: /Create annotation/i }),
      ).toBeInTheDocument();
    });
    const combobox = screen.getByRole("combobox");
    expect(combobox).not.toHaveAttribute("aria-disabled", "true");
    // Submit is disabled until the user enters content
    expect(screen.getByRole("button", { name: /Create/i })).toBeDisabled();
  });

  it("orders annotation chips by the form's domain-annotation order, not by insertion order", async () => {
    // Form has three domain annotations in the order AE, VS, LB.
    // The server delivers three item-level annotations in a different
    // order (LB first, then VS, then AE) — the page must still render
    // them AE → VS → LB.
    const detailWithThree = {
      ...fakeDetail,
      domainAnnotations: [
        {
          id: 50,
          formId: 11,
          name: "AE",
          description: "Adverse Events",
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
        {
          id: 51,
          formId: 11,
          name: "VS",
          description: "Vital Signs",
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
        {
          id: 52,
          formId: 11,
          name: "LB",
          description: "Lab",
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
      ],
      formAnnotations: [
        {
          id: 200,
          domainAnnotationId: 52,
          content: "form-LB",
          assign: false,
          owner: { kind: "form", id: 11 },
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
        {
          id: 201,
          domainAnnotationId: 50,
          content: "form-AE",
          assign: false,
          owner: { kind: "form", id: 11 },
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
        {
          id: 202,
          domainAnnotationId: 51,
          content: "form-VS",
          assign: false,
          owner: { kind: "form", id: 11 },
          createdAt: "2026-01-01T00:00:00Z",
          updatedAt: "2026-01-02T00:00:00Z",
        },
      ],
      items: [
        {
          ...fakeDetail.items[0]!,
          annotations: [
            {
              id: 300,
              domainAnnotationId: 52,
              content: "item-LB",
              assign: true,
              owner: { kind: "item", id: 21 },
              createdAt: "2026-01-01T00:00:00Z",
              updatedAt: "2026-01-02T00:00:00Z",
            },
            {
              id: 301,
              domainAnnotationId: 50,
              content: "item-AE",
              assign: true,
              owner: { kind: "item", id: 21 },
              createdAt: "2026-01-01T00:00:00Z",
              updatedAt: "2026-01-02T00:00:00Z",
            },
            {
              id: 302,
              domainAnnotationId: 51,
              content: "item-VS",
              assign: true,
              owner: { kind: "item", id: 21 },
              createdAt: "2026-01-01T00:00:00Z",
              updatedAt: "2026-01-02T00:00:00Z",
            },
          ],
        },
      ],
    };

    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => detailWithThree,
    });

    renderPage(["/project/abc/crf/11"]);

    // Wait for both form-level and item-level chips to be in the DOM
    await screen.findByText("form-AE");
    await screen.findByText("item-AE");

    // Within their respective chips containers, the order must match
    // the form's domain-annotation order AE → VS → LB, regardless of
    // the order the server delivered them in. The labels live in a
    // `MuiChip-label` span inside each chip root, so we walk up to the
    // chip root before comparing positions.
    const chipPositionsInDoc = (texts: string[]) => {
      const els = texts.map((t) => {
        const label = screen.getByText(t);
        // Each chip renders as `<Chip label="…">` which puts the text
        // in a `<span class="MuiChip-label">`. Walk up to the chip
        // root before comparing positions so chips that wrap their
        // label in extra spans are still ordered by their visual order.
        const root = label.closest(".MuiChip-root");
        if (!root) throw new Error(`no chip root for ${t}`);
        return root;
      });
      // Compare each element against every other element. The number
      // of peers that PRECEDE each chip (DOCUMENT_POSITION_PRECEDING
      // = 2) is its 0-based index in the document order. Asserting on
      // the "preceding" count instead of the "following" count means
      // the first chip has 0, the second 1, the third 2 — matching the
      // intuitive position.
      return els.map((el) => {
        let preceding = 0;
        for (const other of els) {
          if (other === el) continue;
          if (
            (el.compareDocumentPosition(other) &
              Node.DOCUMENT_POSITION_PRECEDING) ===
            Node.DOCUMENT_POSITION_PRECEDING
          ) {
            preceding++;
          }
        }
        return preceding;
      });
    };

    const formOrder = chipPositionsInDoc(["form-AE", "form-VS", "form-LB"]);
    expect(formOrder).toEqual([0, 1, 2]);

    const itemOrder = chipPositionsInDoc(["item-AE", "item-VS", "item-LB"]);
    expect(itemOrder).toEqual([0, 1, 2]);
  });

  it("cascades a domain-annotation delete: deletes every annotation first, then the domain annotation", async () => {
    // Fixture has 1 form-level annotation, 1 item-level annotation,
    // and 1 option-level annotation — all linked to domainAnnotationId 50.
    // A second domain annotation 99 has no annotations, so only 50
    // should trigger the cascade.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      // The cascade delete runs `delete_crf_annotation` once per
      // linked annotation, then `delete_crf_domain_annotation` last.
      // Both succeed silently.
      delete_crf_annotation: () => undefined,
      delete_crf_domain_annotation: () => undefined,
    });

    renderPage(["/project/abc/crf/11"]);

    // The domain-annotation chip is the first chip in the header.
    // MUI Chip renders its delete affordance as a button with the
    // `.MuiChip-deleteIcon` class.
    const chip = await screen.findByTestId("domain-annotation-chip-50");
    const chipRoot = chip.closest(".MuiChip-root")!;
    const deleteIcon = chipRoot.querySelector(".MuiChip-deleteIcon");
    expect(deleteIcon).not.toBeNull();
    fireEvent.click(deleteIcon!);

    // Confirmation dialog appears.
    const confirmButton = await screen.findByRole("button", {
      name: /Delete/i,
    });
    fireEvent.click(confirmButton);

    // After confirmation, the cascade must run:
    //   1. delete every annotation that pointed at domain 50
    //      (here: form 100, item 110 — option has no annotations
    //       linked to 50 in the base fixture, so just those two)
    //   2. then delete the domain annotation 50 itself
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.map((c) => c[0]);
      expect(calls).toContain("delete_crf_annotation");
      expect(calls).toContain("delete_crf_domain_annotation");
    });

    const calls = mockInvoke.mock.calls;
    const deleteAnnCalls = calls
      .filter((c) => c[0] === "delete_crf_annotation")
      .map((c) => c[1]?.id);
    const deleteDomainCalls = calls
      .filter((c) => c[0] === "delete_crf_domain_annotation")
      .map((c) => c[1]?.id);

    expect(deleteAnnCalls).toEqual(expect.arrayContaining([100, 110]));
    expect(deleteDomainCalls).toEqual([50]);

    // The annotation deletes must precede the domain delete — the
    // mutation is sequential so a halfway failure surfaces to the user
    // rather than corrupting the cache with a half-deleted cascade.
    const lastAnnotationIdx = calls
      .map((c) => c[0])
      .lastIndexOf("delete_crf_annotation");
    const firstDomainIdx = calls.findIndex(
      (c) => c[0] === "delete_crf_domain_annotation",
    );
    expect(lastAnnotationIdx).toBeLessThan(firstDomainIdx);
  });

  it("renders a [NOT SUBMITTED] chip on the form / item / option / unit when the flag is true", async () => {
    // Same fixture as the basic render test, but every entity
    // (form, item, option, unit) has notSubmitted flipped to true.
    const detailNotSubmitted = {
      ...fakeDetail,
      form: { ...fakeDetail.form, notSubmitted: true },
      items: fakeDetail.items.map((item) => ({
        ...item,
        item: { ...item.item, notSubmitted: true },
        options: item.options.map((o) => ({
          ...o,
          option: { ...o.option, notSubmitted: true },
        })),
        units: item.units.map((u) => ({
          ...u,
          unit: { ...u.unit, notSubmitted: true },
        })),
      })),
    };
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted: true }),
      get_crf_form_details: () => detailNotSubmitted,
    });

    renderPage(["/project/abc/crf/11"]);

    // Wait for the page to render, then count [NOT SUBMITTED] chips.
    await screen.findByTestId("crf-form-name");
    const chips = await screen.findAllByTestId("not-submitted-chip");
    // 1 form + 1 item + 1 option + 1 unit = 4 chips
    expect(chips).toHaveLength(4);
    expect(chips.every((c) => c.textContent === "[NOT SUBMITTED]")).toBe(true);
  });

  it("cascades annotations when toggling `Not submitted` on the form via the DomainAnnotationDialog", async () => {
    // Open the DomainAnnotationDialog for the AE domain annotation,
    // check the new `Not submitted` checkbox, and save. The page
    // must:
    //   1. save the domain annotation (the dialog handles the name /
    //      description save)
    //   2. run the not-submitted cascade against the form: delete
    //      every annotation attached to the form (here: form 100
    //      + item 110), then PATCH the form's notSubmitted=true.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      update_crf_domain_annotation: () => ({
        id: 50,
        formId: 11,
        name: "AE",
        description: "Adverse Events",
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-02T00:00:00Z",
      }),
      update_crf_form: () => fakeForm,
      delete_crf_annotation: () => undefined,
    });

    renderPage(["/project/abc/crf/11"]);

    // Open the domain annotation dialog by clicking the chip.
    const chip = await screen.findByTestId("domain-annotation-chip-50");
    fireEvent.click(chip);

    // The dialog pre-fills with the row's data; the new `Not
    // submitted` checkbox should be unchecked (current form flag
    // is false in the fixture).
    const checkboxes = await screen.findAllByRole("checkbox");
    // The dialog only has one checkbox in edit mode: `Not submitted`.
    // (`Assigned` lives on the annotation dialog, not the domain one.)
    expect(checkboxes).toHaveLength(1);
    expect(checkboxes[0]).not.toBeChecked();
    fireEvent.click(checkboxes[0]);

    // Submit.
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));

    // After submit, the page must have:
    //   - updated the domain annotation (PATCH)
    //   - deleted the form's annotations (annotation 100 + item
    //     annotation 110 in this fixture)
    //   - PATCHed the form with notSubmitted=true
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.map((c) => c[0]);
      expect(calls).toContain("update_crf_domain_annotation");
      expect(calls).toContain("delete_crf_annotation");
      expect(calls).toContain("update_crf_form");
    });

    const calls = mockInvoke.mock.calls;
    const deleteAnnIds = calls
      .filter((c) => c[0] === "delete_crf_annotation")
      .map((c) => c[1]?.id);
    expect(deleteAnnIds).toEqual(expect.arrayContaining([100, 110]));

    const updateFormCalls = calls.filter((c) => c[0] === "update_crf_form");
    expect(updateFormCalls).toHaveLength(1);
    expect(updateFormCalls[0]?.[1]).toMatchObject({
      id: 11,
      body: { notSubmitted: true },
    });

    // Annotations must be deleted before the form's notSubmitted
    // flag is updated — otherwise a halfway failure would leave
    // dangling annotations on a "not submitted" form.
    const lastAnnotationIdx = calls
      .map((c) => c[0])
      .lastIndexOf("delete_crf_annotation");
    const firstFormIdx = calls.findIndex((c) => c[0] === "update_crf_form");
    expect(lastAnnotationIdx).toBeLessThan(firstFormIdx);
  });
});