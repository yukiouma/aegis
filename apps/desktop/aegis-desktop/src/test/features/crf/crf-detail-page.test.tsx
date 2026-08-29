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
});