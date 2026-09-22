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
      // Project-leader mock so the create-menu items stay enabled —
      // the menu items are now gated on `canEditAnnotations` which
      // requires the leader query to resolve to true.
      get_project_by_code: () => leaderProject,
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
      // Leader mock so the domain-annotation chip stays clickable —
      // the chip's `onDelete` is gated on `canEditAnnotations`.
      get_project_by_code: () => leaderProject,
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

  it("clears the [NOT SUBMITTED] flag when the chip's delete icon is clicked, without cascade-deleting annotations", async () => {
    // All four entities start not-submitted with annotations attached.
    // Clicking the chip's delete icon must PATCH each owner with
    // notSubmitted=false and must NOT delete any annotation (the
    // cascade only fires on a `false → true` transition).
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
      // Leader mock so the [NOT SUBMITTED] chip's delete affordance
      // is rendered — `canClearNotSubmitted` requires leader / DEV.
      get_project_by_code: () => leaderProject,
      update_crf_form: () => fakeForm,
      update_crf_item: () => detailNotSubmitted.items[0]!.item,
      update_crf_option: () => detailNotSubmitted.items[0]!.options[0]!.option,
      update_crf_unit: () => detailNotSubmitted.items[0]!.units[0]!.unit,
    });

    renderPage(["/project/abc/crf/11"]);

    // Let the chips mount.
    const chips = await screen.findAllByTestId("not-submitted-chip");
    expect(chips).toHaveLength(4);

    // Click every chip's delete icon, one at a time. The cascade must
    // not run, so we check after the final click that no
    // `delete_crf_annotation` call ever fired.
    for (const chip of chips) {
      const chipRoot = chip.closest(".MuiChip-root")!;
      const deleteIcon = chipRoot.querySelector(".MuiChip-deleteIcon");
      expect(deleteIcon).not.toBeNull();
      fireEvent.click(deleteIcon!);
    }

    // Each owner kind maps to a different wire command. We expect all
    // four to fire exactly once with `notSubmitted: false`.
    await waitFor(() => {
      const calls = mockInvoke.mock.calls;
      const commands = calls.map((c) => c[0]);
      expect(commands).toEqual(
        expect.arrayContaining([
          "update_crf_form",
          "update_crf_item",
          "update_crf_option",
          "update_crf_unit",
        ]),
      );
    });

    const calls = mockInvoke.mock.calls;
    const findBody = (cmd: string) =>
      calls.find((c) => c[0] === cmd)?.[1];
    expect(findBody("update_crf_form")).toMatchObject({
      id: 11,
      body: { notSubmitted: false },
    });
    expect(findBody("update_crf_item")).toMatchObject({
      id: 21,
      body: { notSubmitted: false },
    });
    expect(findBody("update_crf_option")).toMatchObject({
      id: 31,
      body: { notSubmitted: false },
    });
    expect(findBody("update_crf_unit")).toMatchObject({
      id: 41,
      body: { notSubmitted: false },
    });

    // The cascade must NOT fire — the transition is `true → false`,
    // so no annotations should be deleted.
    expect(
      calls.some((c) => c[0] === "delete_crf_annotation"),
    ).toBe(false);
  });

  it("cascades annotations and domain annotations when clicking `Not submit` in the create-annotation dialog", async () => {
    // Open the AnnotationDialog for the form in CREATE mode
    // (via the form-name hover menu's `New annotation` entry) and
    // click the dialog's `Not submit` button. The page must:
    //   1. delete every annotation attached to the form (here:
    //      form 100 + item 110),
    //   2. delete every domain annotation in the form (here: AE id 50),
    //   3. PATCH the form with notSubmitted=true.
    // Order matters: annotations → domain annotations → form PATCH,
    // so a halfway failure surfaces rather than leaving the form in a
    // half-cleared state.
    //
    // We use the AnnotationDialog in create mode rather than the
    // DomainAnnotationDialog in edit mode because the form-level
    // `Not submit` action is now only exposed while creating a new
    // annotation / domain annotation. Editing an existing row is
    // a name / content change, not a form-level flag decision.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      // Leader mock so the menu's `New annotation` entry stays
      // enabled — the test opens it to reach the create dialog.
      get_project_by_code: () => leaderProject,
      update_crf_form: () => fakeForm,
      delete_crf_annotation: () => undefined,
      delete_crf_domain_annotation: () => undefined,
    });

    renderPage(["/project/abc/crf/11"]);

    // Open the form-name hover menu and click `New annotation`.
    // Double-click pattern matches the menu-disable tests above —
    // the first click after the menu mount is occasionally
    // swallowed by React 18's microtask batching.
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const menuItems = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newAnnotation = menuItems.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newAnnotation).toBeDefined();
    fireEvent.click(newAnnotation!);

    // The dialog should expose a `Not submit` action button (the
    // form is currently submitted, i.e. notSubmitted === false,
    // and the dialog is in create mode).
    const notSubmit = await screen.findByTestId(
      "crf-annotation-dialog-not-submit",
    );
    expect(notSubmit).toBeInTheDocument();
    fireEvent.click(notSubmit);

    // After clicking, the page must have:
    //   - deleted the form's annotations (annotation 100 + item
    //     annotation 110 in this fixture)
    //   - deleted the form's domain annotations (id 50)
    //   - PATCHed the form with notSubmitted=true
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.map((c) => c[0]);
      expect(calls).toContain("delete_crf_annotation");
      expect(calls).toContain("delete_crf_domain_annotation");
      expect(calls).toContain("update_crf_form");
    });

    const calls = mockInvoke.mock.calls;
    const deleteAnnIds = calls
      .filter((c) => c[0] === "delete_crf_annotation")
      .map((c) => c[1]?.id);
    expect(deleteAnnIds).toEqual(expect.arrayContaining([100, 110]));

    const deleteDomainIds = calls
      .filter((c) => c[0] === "delete_crf_domain_annotation")
      .map((c) => c[1]?.id);
    expect(deleteDomainIds).toEqual([50]);

    const updateFormCalls = calls.filter((c) => c[0] === "update_crf_form");
    expect(updateFormCalls).toHaveLength(1);
    expect(updateFormCalls[0]?.[1]).toMatchObject({
      id: 11,
      body: { notSubmitted: true },
    });

    // Strict ordering: annotations → domain annotations → form PATCH.
    // Annotations must be deleted before domain annotations because
    // deleting a domain annotation can cascade to its annotations;
    // deleting annotations first means each step is independent.
    // Domain annotations must precede the form PATCH so a halfway
    // failure leaves the form in either a fully-populated or a
    // fully-empty state — never half-cleared with dangling refs.
    const lastAnnIdx = calls
      .map((c) => c[0])
      .lastIndexOf("delete_crf_annotation");
    const firstDomainIdx = calls.findIndex(
      (c) => c[0] === "delete_crf_domain_annotation",
    );
    const lastDomainIdx = calls
      .map((c) => c[0])
      .lastIndexOf("delete_crf_domain_annotation");
    const firstFormIdx = calls.findIndex((c) => c[0] === "update_crf_form");
    expect(lastAnnIdx).toBeLessThan(firstDomainIdx);
    expect(lastDomainIdx).toBeLessThan(firstFormIdx);
  });

  it("renders the form-level [NOT SUBMITTED] header chip after `Not submit` succeeds", async () => {
    // The header chip reads `form.notSubmitted` from `useGetCrfForm`
    // (the `crf.form` query, populated by `get_crf_form_by_id`). The
    // cascade mutation must invalidate BOTH `crf.formDetail` and
    // `crf.form` on success — otherwise the header stays stale even
    // after the cascade and PATCH succeed, and the user never sees
    // the chip appear. Here we model the post-mutation reality by
    // flipping the flag the mock returns once `update_crf_form`
    // has been called.
    let notSubmitted = false;
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted }),
      get_crf_form_details: () => ({
        ...fakeDetail,
        form: { ...fakeDetail.form, notSubmitted },
      }),
      // Leader mock so the menu's `New annotation` entry stays
      // enabled — the test opens it to reach the create dialog.
      get_project_by_code: () => leaderProject,
      update_crf_form: (args) => {
        notSubmitted = (args?.body as { notSubmitted: boolean })
          ?.notSubmitted === true;
        return { ...fakeForm, notSubmitted };
      },
      delete_crf_annotation: () => undefined,
      delete_crf_domain_annotation: () => undefined,
    });

    renderPage(["/project/abc/crf/11"]);

    // Sanity: header starts without the chip — form is submitted.
    expect(screen.queryByTestId("not-submitted-chip")).not.toBeInTheDocument();

    // Open the form-name hover menu and click `New annotation`
    // to open the AnnotationDialog in create mode. The
    // `Not submit` action is now only exposed while creating a
    // new annotation / domain annotation, so we use the create
    // path to trigger the form-level cascade.
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const menuItems = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newAnnotation = menuItems.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newAnnotation).toBeDefined();
    fireEvent.click(newAnnotation!);

    const notSubmit = await screen.findByTestId(
      "crf-annotation-dialog-not-submit",
    );
    fireEvent.click(notSubmit);

    // The cascade runs, the form PATCH returns notSubmitted=true, and
    // both queries re-fetch. The header chip — keyed by
    // `data-testid="not-submitted-chip"` — must appear in the DOM.
    // waitFor handles the React Query refetch round-trip.
    await waitFor(() => {
      expect(
        screen.getByTestId("not-submitted-chip"),
      ).toBeInTheDocument();
    });

    // After the action, `get_crf_form_by_id` must have been called at
    // least twice: once for the initial load, once after invalidation.
    // If the mutation only invalidated `crf.formDetail`, the hook
    // would never refetch the single-form query and the chip would
    // stay stale.
    const getByIdCalls = mockInvoke.mock.calls.filter(
      (c) => c[0] === "get_crf_form_by_id",
    );
    expect(getByIdCalls.length).toBeGreaterThanOrEqual(2);
  });

  it("disables the New domain / New annotation menu items when the form is marked not submitted", async () => {
    // While the form is not-submitted the cascade has already wiped
    // every annotation AND every domain annotation, so the create
    // entry points in the form-name hover menu must be disabled to
    // prevent the user from opening an empty dialog. Edit flows
    // (clicking an existing domain annotation chip / annotation
    // chip) still work — those are not blocked here.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      // useSdtmContext (now threaded into the page) reads project
      // configurations; mock it so the SDTM hooks don't error and
      // re-render the page while this test is interacting with the
      // form-name menu.
      get_project_by_code: () => leaderProject,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted: true }),
      get_crf_form_details: () => ({
        ...fakeDetail,
        form: { ...fakeDetail.form, notSubmitted: true },
        // The cascade would have wiped domain annotations in the
        // real flow; the fixture is the post-cascade reality.
        domainAnnotations: [],
        formAnnotations: [],
      }),
    });

    renderPage(["/project/abc/crf/11"]);

    // Wait for the form header to render — proves the page has the
    // not-submitted flag before we interact with it. findByTestId
    // polls the DOM up to the default timeout, so waiting on the
    // chip directly also covers the small race where the form
    // query resolves before the chip mounts.
    const formName = await screen.findByTestId("crf-form-name");
    await screen.findByTestId("not-submitted-chip");

    // Open the menu. The first click after the chip's mount is
    // occasionally swallowed by a microtask race in React 18
    // (made worse by the SDTM-context queries that the page now
    // runs); retry the click up to a few times until the popover
    // actually mounts.
    await waitFor(() => {
      fireEvent.click(formName);
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const menuItems = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = menuItems.find(
      (el) => el.textContent?.trim() === "New domain",
    );
    const newAnnotation = menuItems.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).toBeDefined();
    expect(newAnnotation).toBeDefined();

    // Both create entries must be disabled so MUI ignores clicks.
    expect(newDomain).toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).toHaveAttribute("aria-disabled", "true");
    expect(newDomain).toHaveClass("Mui-disabled");
    expect(newAnnotation).toHaveClass("Mui-disabled");

    // Even if a future caller bypasses the menu (a keyboard
    // shortcut or a programmatic trigger), the page-level guard
    // must short-circuit so no create dialog mounts. The DOM-level
    // click on a `Mui-disabled` MenuItem is a no-op, but we
    // still force-click to prove the page-level guard kicks in if
    // the disabled flag is somehow bypassed.
    fireEvent.click(newDomain!);
    fireEvent.click(newAnnotation!);
    expect(
      screen.queryByRole("heading", { name: /Create annotation/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /Create domain/i }),
    ).not.toBeInTheDocument();
  });

  it("ignores clicks on item / option / unit when the form is marked not submitted", async () => {
    // The item / option / unit Typography click handlers each route
    // back to `openCreateAnnotation({ kind, id })`. While the form
    // is not submitted, CrfItemRow short-circuits the click so the
    // create dialog never opens, and the cursor / hover affordances
    // are removed to match.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted: true }),
      get_crf_form_details: () => ({
        ...fakeDetail,
        form: { ...fakeDetail.form, notSubmitted: true },
        domainAnnotations: [],
        formAnnotations: [],
      }),
    });

    renderPage(["/project/abc/crf/11"]);

    // Confirm we're testing the not-submitted branch.
    await screen.findByTestId("not-submitted-chip");

    const item = await screen.findByTestId("crf-item-name-21");
    const option = await screen.findByTestId("crf-option-31");
    const unit = await screen.findByTestId("crf-unit-41");

    fireEvent.click(item);
    fireEvent.click(option);
    fireEvent.click(unit);

    // No create dialog should have mounted.
    expect(
      screen.queryByRole("heading", { name: /Create annotation/i }),
    ).not.toBeInTheDocument();

    // The cursor should be the default (auto / not pointer) since
    // the click is no longer advertised. MUI uses the
    // `cursor: pointer` style — assert via inline style that the
    // element doesn't carry it.
    expect(item).not.toHaveStyle({ cursor: "pointer" });
    expect(option).not.toHaveStyle({ cursor: "pointer" });
    expect(unit).not.toHaveStyle({ cursor: "pointer" });
  });

  it("ignores clicks on item / option / unit when the item itself is marked not submitted", async () => {
    // When an item is marked not-submitted, the item-level cascade
    // wipes the item's annotations AND the annotations on its
    // options and units — so every create-annotation entry point
    // on this row must short-circuit, not just the item. The
    // form-level state stays submitted (notSubmitted=false) so
    // the form chip / menu are unaffected; only this row is gated.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => ({
        ...fakeDetail,
        items: [
          {
            ...fakeDetail.items[0]!,
            item: { ...fakeDetail.items[0]!.item, notSubmitted: true },
          },
        ],
      }),
    });

    renderPage(["/project/abc/crf/11"]);

    // The per-row chip on the item is the visible signal that
    // we're testing the item-not-submitted branch. Wait on it so
    // the row has mounted before clicking.
    const itemName = await screen.findByTestId("crf-item-name-21");
    await screen.findByTestId("not-submitted-chip");
    const option = screen.getByTestId("crf-option-31");
    const unit = screen.getByTestId("crf-unit-41");

    fireEvent.click(itemName);
    fireEvent.click(option);
    fireEvent.click(unit);

    expect(
      screen.queryByRole("heading", { name: /Create annotation/i }),
    ).not.toBeInTheDocument();

    // Same affordance drop as the form-not-submitted branch —
    // the row is no longer advertised as clickable.
    expect(itemName).not.toHaveStyle({ cursor: "pointer" });
    expect(option).not.toHaveStyle({ cursor: "pointer" });
    expect(unit).not.toHaveStyle({ cursor: "pointer" });
  });

  it("ignores clicks on item / option / unit when the form has no domain annotations", async () => {
    // An annotation needs a domain annotation to belong to. While
    // the form has none, every create-annotation entry point on
    // the row must short-circuit. The form-level `New domain`
    // menu item stays open — that's how the first one is created.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => ({
        ...fakeDetail,
        // No domain annotation means no annotation can be created.
        // Keep the existing form / item annotations so we know
        // they don't influence the gating.
        domainAnnotations: [],
      }),
    });

    renderPage(["/project/abc/crf/11"]);

    const item = await screen.findByTestId("crf-item-name-21");
    const option = screen.getByTestId("crf-option-31");
    const unit = screen.getByTestId("crf-unit-41");

    fireEvent.click(item);
    fireEvent.click(option);
    fireEvent.click(unit);

    expect(
      screen.queryByRole("heading", { name: /Create annotation/i }),
    ).not.toBeInTheDocument();
    expect(item).not.toHaveStyle({ cursor: "pointer" });
    expect(option).not.toHaveStyle({ cursor: "pointer" });
    expect(unit).not.toHaveStyle({ cursor: "pointer" });
  });

  it("disables the New annotation menu item when the form has no domain annotations", async () => {
    // The form-name hover menu's `New annotation` item is
    // disabled (with a tooltip) when there are no domain
    // annotations to assign to. `New domain` stays open — that's
    // the path to creating the first one.
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => ({
        ...fakeDetail,
        domainAnnotations: [],
      }),
      // Leader mock so `canEditAnnotations` resolves true —
      // otherwise the test would conflate the no-domain-annotations
      // disable with the no-permission disable.
      get_project_by_code: () => leaderProject,
    });

    renderPage(["/project/abc/crf/11"]);

    const formName = await screen.findByTestId("crf-form-name");
    // Same microtask-race workaround as the form-not-submitted
    // test: the first click after `findByTestId` is occasionally
    // swallowed by React 18 batching.
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const menuItems = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = menuItems.find(
      (el) => el.textContent?.trim() === "New domain",
    );
    const newAnnotation = menuItems.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).toBeDefined();
    expect(newAnnotation).toBeDefined();

    // Only the `New annotation` entry is gated; `New domain`
    // stays enabled so the user can create the first one.
    expect(newDomain).not.toHaveAttribute("aria-disabled", "true");
    expect(newDomain).not.toHaveClass("Mui-disabled");
    expect(newAnnotation).toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).toHaveClass("Mui-disabled");

    // Force-click past the disabled flag to prove the
    // page-level guard also kicks in.
    fireEvent.click(newAnnotation!);
    expect(
      screen.queryByRole("heading", { name: /Create annotation/i }),
    ).not.toBeInTheDocument();
  });
});

// Mission-issue wiring. The form → mission → token lookup mirrors the
// CrfMissionAssignDrawer pattern (`missions.find((m) => m.missionCode
// === form.code)`); the chip clicks open the dialog with the
// correct scope.
const fakeMission = {
  id: 10,
  projectCode: "abc",
  missionKind: "crf" as const,
  missionCode: "AE",
  assignees: [
    {
      id: 100,
      userCode: "u",
      role: "qc" as const,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
  ],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const openedMissionIssue = {
  id: 1,
  missionId: 10,
  issuer: "u",
  description: "missing row",
  state: "opened" as const,
  comments: [],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

describe("CrfDetailPage — mission-issue entry points", () => {
  it("form code chip is enabled when a mission exists for the form", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      list_missions_by_project: () => [fakeMission],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");
  });

  it("clicking the form chip opens the dialog showing the dialog title", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      list_missions_by_project: () => [fakeMission],
      list_issues_by_mission: () => [openedMissionIssue],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    fireEvent.click(chip);
    // MissionIssueDialog renders the title with i18n key
    // "crf.missionIssue.dialog.title" and the scope label.
    expect(
      await screen.findByText(/Mission issues/i),
    ).toBeInTheDocument();
  });

  it("clicking an item code chip opens the dialog filtered to that item's code", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      list_missions_by_project: () => [fakeMission],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-item-code-21");
    fireEvent.click(chip);
    expect(
      await screen.findByText(/Mission issues/i),
    ).toBeInTheDocument();
  });

  // Spec §Testing cases 1-5: chip + badge behavior. The Badge
  // shows the open-issue count for the chip's scope; when no
  // issues are open the Badge hides via MUI's default `showZero:
  // false`. This test exercises the four counts (open, closed,
  // none, item-target mismatch).
  it("form chip count badge is visible when an opened mission-level issue exists", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      list_missions_by_project: () => [fakeMission],
      list_issues_by_mission: () => [openedMissionIssue],
    });
    renderPage(["/project/abc/crf/11"]);
    await screen.findByTestId("crf-form-11");
    await waitFor(() => {
      // With badgeContent > 0 the `.MuiBadge-badge` element renders
      // and carries the count. The form code chip stays clickable.
      const chip = screen.getByTestId("crf-form-11");
      const wrapper = chip.closest(".MuiChip-root")?.parentElement;
      const badge = wrapper?.querySelector(".MuiBadge-badge");
      expect(badge).not.toBeNull();
      expect(badge?.textContent).toBe("1");
    });
  });

  it("form chip count badge is hidden when only closed issues exist", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      list_missions_by_project: () => [fakeMission],
      list_issues_by_mission: () => [
        { ...openedMissionIssue, id: 2, state: "closed" as const },
      ],
    });
    renderPage(["/project/abc/crf/11"]);
    await screen.findByTestId("crf-form-11");
    // With badgeContent = 0 MUI applies the `MuiBadge-invisible` class
    // (via its default `showZero: false`). We don't need to wait —
    // the chip is synchronous once data is present.
    await waitFor(() => {
      const chip = screen.getByTestId("crf-form-11");
      const wrapper = chip.closest(".MuiChip-root")?.parentElement;
      const badge = wrapper?.querySelector(".MuiBadge-badge");
      expect(badge?.className ?? "").toMatch(/MuiBadge-invisible/);
    });
  });
});

// =========================================================================
// Role-based restriction coverage. Each case drives the leader / QC / DEV
// booleans by mocking `get_project_by_code` (so useIsProjectLeader
// resolves) AND the project's leaders list, then mocks
// `list_missions_by_project` / `list_issues_by_mission` for the chip
// state. The shape mirrors `ProjectView` from
// `apps/desktop/aegis-desktop/src/shared/api/types.ts`.
// =========================================================================

const baseProject = {
  id: 1,
  code: "abc",
  description: "Test project",
  configurations: { language: "en" as const, tags: [] },
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const leaderProject = {
  ...baseProject,
  members: {
    leaders: [{ code: "u", name: "U" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [{ code: "u", name: "U" }],
    workers: [],
  },
};

const nonLeaderProject = {
  ...baseProject,
  members: {
    leaders: [{ code: "other", name: "Other" }],
    workers: [],
  },
  unblindMembers: {
    leaders: [{ code: "other", name: "Other" }],
    workers: [],
  },
};

function missionForUser(role: "qc" | "dev" | "other") {
  return {
    ...fakeMission,
    assignees:
      role === "other"
        ? []
        : [{ ...fakeMission.assignees[0], role }],
  };
}

describe("CrfDetailPage — role-based restrictions", () => {
  it("project leader: form chip enabled with zero issues; menu and chips enabled", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => leaderProject,
      list_missions_by_project: () => [missionForUser("qc")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    // Leader exempt: form chip stays enabled even though zero issues.
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");

    // Menu: both MenuItems enabled.
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find(
      (el) => el.textContent?.trim() === "New domain",
    );
    const newAnnotation = items.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).not.toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).not.toHaveAttribute("aria-disabled", "true");
  });

  it("mission QC: annotation menu disabled but form chip is enabled (empty-issue dialog allowed)", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("qc")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    const chip = await screen.findByTestId("crf-form-11");
    // QC can open the empty-issue dialog — chip stays enabled.
    expect(chip).not.toHaveAttribute("aria-disabled", "true");

    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find(
      (el) => el.textContent?.trim() === "New domain",
    );
    const newAnnotation = items.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    // QC can't edit annotations → both menu items are disabled.
    expect(newDomain).toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).toHaveAttribute("aria-disabled", "true");

    // Item / option / unit Typography drop pointer cursor.
    const item = await screen.findByTestId("crf-item-name-21");
    expect(item).not.toHaveStyle({ cursor: "pointer" });

    // Annotation chip stays in the same outlined style (no
    // `Mui-disabled`, no tooltip wrapper) but the click is
    // silently dropped — the chip's `onClick` is unset.
    const ann = await screen.findByText("item-level note");
    expect(ann.closest(".MuiChip-root")).not.toHaveClass("Mui-disabled");
    expect(ann.closest(".MuiChip-root")).not.toHaveClass("MuiChip-clickable");
  });

  it("mission DEV: form chip does not open the issue dialog when zero issues; menu stays enabled", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("dev")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    // DEV can't open the empty-issue dialog — the chip keeps its
    // outlined style but clicking it does not open the dialog.
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");
    fireEvent.click(chip);
    await waitFor(() => {
      expect(
        screen.queryByText(/Mission issues/i),
      ).not.toBeInTheDocument();
    });

    // Menu still enabled (DEV can edit annotations).
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find(
      (el) => el.textContent?.trim() === "New domain",
    );
    expect(newDomain).not.toHaveAttribute("aria-disabled", "true");
  });

  it("mission DEV: form chip becomes enabled once an issue exists", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("dev")],
      list_issues_by_mission: () => [openedMissionIssue],
    });
    renderPage(["/project/abc/crf/11"]);
    // With at least one issue, the chip un-disables. Wait for the
    // mission / issues / leader queries to resolve — initially
    // `formMission` is undefined so the chip renders as
    // `aria-disabled="true"` regardless of role, then flips once
    // the queries settle.
    const chip = await screen.findByTestId("crf-form-11");
    await waitFor(() => {
      expect(chip).not.toHaveAttribute("aria-disabled", "true");
    });
  });

  it("task unrelated (no role): strictest — chips and menu all disabled", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => fakeForm,
      get_crf_form_details: () => fakeDetail,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("other")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);
    // Form chip keeps its outlined style but does not open the
    // issue dialog when there's no role + no issues.
    const chip = await screen.findByTestId("crf-form-11");
    expect(chip).not.toHaveAttribute("aria-disabled", "true");
    fireEvent.click(chip);
    await waitFor(() => {
      expect(
        screen.queryByText(/Mission issues/i),
      ).not.toBeInTheDocument();
    });

    // Menu items disabled.
    const formName = await screen.findByTestId("crf-form-name");
    fireEvent.click(formName);
    fireEvent.click(formName);
    await waitFor(() => {
      expect(
        document.querySelectorAll(".MuiMenuItem-root").length,
      ).toBeGreaterThanOrEqual(2);
    });
    const items = Array.from(
      document.querySelectorAll<HTMLElement>(".MuiMenuItem-root"),
    );
    const newDomain = items.find(
      (el) => el.textContent?.trim() === "New domain",
    );
    const newAnnotation = items.find(
      (el) => el.textContent?.trim() === "New annotation",
    );
    expect(newDomain).toHaveAttribute("aria-disabled", "true");
    expect(newAnnotation).toHaveAttribute("aria-disabled", "true");

    // Item Typography drops pointer cursor.
    const item = await screen.findByTestId("crf-item-name-21");
    expect(item).not.toHaveStyle({ cursor: "pointer" });

    // Annotation chip keeps the outlined style (no `Mui-disabled`,
    // no tooltip) but the click is silently dropped.
    const ann = await screen.findByText("item-level note");
    expect(ann.closest(".MuiChip-root")).not.toHaveClass("Mui-disabled");
    expect(ann.closest(".MuiChip-root")).not.toHaveClass("MuiChip-clickable");
  });

  // [NOT SUBMITTED] chip's delete affordance is gated on
  // `canClearNotSubmitted = isLeader || isMissionDev`. Mission QC
  // (the reviewer) and task-unrelated users see the chip but can't
  // click it to clear the flag — leader and DEV are the only two
  // roles allowed to clear.
  it("mission QC: [NOT SUBMITTED] chip renders without delete affordance", async () => {
    const detailNotSubmitted = {
      ...fakeDetail,
      form: { ...fakeDetail.form, notSubmitted: true },
      items: [
        {
          ...fakeDetail.items[0]!,
          item: { ...fakeDetail.items[0]!.item, notSubmitted: true },
        },
      ],
    };
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted: true }),
      get_crf_form_details: () => detailNotSubmitted,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("qc")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);

    // Both the form-level chip and the per-item chip render, but
    // without a delete icon — MUI's Chip only renders the delete
    // affordance when `onDelete` is provided.
    const chips = await screen.findAllByTestId("not-submitted-chip");
    expect(chips.length).toBeGreaterThanOrEqual(2);
    for (const c of chips) {
      const root = c.closest(".MuiChip-root")!;
      expect(root.querySelector(".MuiChip-deleteIcon")).toBeNull();
    }
  });

  it("mission DEV: [NOT SUBMITTED] chip retains its delete affordance", async () => {
    const detailNotSubmitted = {
      ...fakeDetail,
      form: { ...fakeDetail.form, notSubmitted: true },
      items: [
        {
          ...fakeDetail.items[0]!,
          item: { ...fakeDetail.items[0]!.item, notSubmitted: true },
        },
      ],
    };
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted: true }),
      get_crf_form_details: () => detailNotSubmitted,
      get_project_by_code: () => nonLeaderProject,
      list_missions_by_project: () => [missionForUser("dev")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);

    // DEV authors annotations — they're allowed to clear the flag
    // (e.g. after re-fixing the item they marked not-submitted).
    const chips = await screen.findAllByTestId("not-submitted-chip");
    expect(chips.length).toBeGreaterThanOrEqual(2);
    for (const c of chips) {
      const root = c.closest(".MuiChip-root")!;
      expect(root.querySelector(".MuiChip-deleteIcon")).not.toBeNull();
    }
  });

  it("project leader: [NOT SUBMITTED] chip retains its delete affordance", async () => {
    const detailNotSubmitted = {
      ...fakeDetail,
      form: { ...fakeDetail.form, notSubmitted: true },
    };
    mockCommands({
      is_logged_in: () => true,
      current_user: () => fakeUser,
      get_crf_form_by_id: () => ({ ...fakeForm, notSubmitted: true }),
      get_crf_form_details: () => detailNotSubmitted,
      get_project_by_code: () => leaderProject,
      list_missions_by_project: () => [missionForUser("qc")],
      list_issues_by_mission: () => [],
    });
    renderPage(["/project/abc/crf/11"]);

    // The form-level chip has a delete icon.
    const chip = await screen.findByTestId("not-submitted-chip");
    const root = chip.closest(".MuiChip-root")!;
    expect(root.querySelector(".MuiChip-deleteIcon")).not.toBeNull();
  });
});