import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import type { UserView } from "../../api";
import { UserTable } from "../../pages/UserTable";

/**
 * MUI v9 Switch renders the underlying <input type="checkbox"> inside
 * a wrapper span but does not always expose `role="checkbox"` via
 * `getByRole`. Query the inputs directly.
 */
function getSwitches(): HTMLInputElement[] {
  return Array.from(
    document.querySelectorAll('input[type="checkbox"]'),
  ) as HTMLInputElement[];
}

function renderTable(
  props: Partial<React.ComponentProps<typeof UserTable>> = {},
) {
  const baseProps = {
    rows: [] as UserView[],
    loading: false,
    mutationLoading: false,
    error: null,
    selfCode: null,
    onToggle: vi.fn(),
    onRoleChange: vi.fn(),
    onRetry: vi.fn(),
  };
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <UserTable {...baseProps} {...props} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

const adminUser: UserView = {
  id: 1,
  code: "alice",
  name: "Alice",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const generalUser: UserView = {
  id: 2,
  code: "bob",
  name: "Bob",
  role: "general",
  active: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};
const adminUser2: UserView = {
  id: 3,
  code: "carol",
  name: "Carol",
  role: "admin",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

afterEach(() => cleanup());

describe("UserTable — rendering states", () => {
  it("renders the spinner when loading with no rows", () => {
    renderTable({ rows: [], loading: true });
    expect(screen.getByRole("progressbar")).toBeInTheDocument();
  });

  it("renders the empty-state copy when no rows and not loading", () => {
    renderTable({ rows: [], loading: false });
    expect(screen.getByText(/no users yet/i)).toBeInTheDocument();
  });

  it("renders emptyMessage when provided and rows is empty", () => {
    renderTable({ rows: [], emptyMessage: "Nothing matched" });
    expect(screen.getByText("Nothing matched")).toBeInTheDocument();
    expect(screen.queryByText(/no users yet/i)).not.toBeInTheDocument();
  });

  it("renders the error Alert when error is set", () => {
    renderTable({
      error: { kind: "http", status: 500, code: "boom", message: "boom" },
    });
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
  });

  it("calls onRetry when the Retry button is clicked", async () => {
    const onRetry = vi.fn();
    renderTable({
      error: { kind: "http", status: 500, code: "boom", message: "boom" },
      onRetry,
    });
    await userEvent.click(screen.getByRole("button", { name: /retry/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});

describe("UserTable — rows", () => {
  it("renders one row per user with code, name, role select, and switch", () => {
    renderTable({ rows: [adminUser, generalUser, adminUser2] });
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText("Alice")).toBeInTheDocument();
    // Each non-root user renders a Select with their current role as
    // the visible label. Two admins + one general.
    const selects = screen.getAllByRole("combobox");
    expect(selects).toHaveLength(3);
    expect(selects[0]).toHaveTextContent("Admin");   // alice
    expect(selects[1]).toHaveTextContent("General"); // bob
    expect(selects[2]).toHaveTextContent("Admin");   // carol
    expect(screen.getByText("bob")).toBeInTheDocument();
    expect(screen.getByText("carol")).toBeInTheDocument();
  });

  it("reflects active=true as a checked Switch", () => {
    renderTable({ rows: [adminUser] });
    const sw = getSwitches()[0];
    expect(sw.checked).toBe(true);
  });

  it("reflects active=false as an unchecked Switch", () => {
    renderTable({ rows: [generalUser] });
    const sw = getSwitches()[0];
    expect(sw.checked).toBe(false);
  });
});

describe("UserTable — role Select", () => {
  /** Find the combobox whose visible text equals `label`. */
  function selectWithLabel(label: string): HTMLElement {
    const selects = screen.getAllByRole("combobox");
    const match = selects.find((s) => s.textContent === label);
    if (!match) throw new Error(`Select with label "${label}" not found`);
    return match;
  }

  it("Select on the self row is disabled", () => {
    renderTable({ rows: [adminUser], selfCode: "alice" });
    expect(selectWithLabel("Admin")).toHaveAttribute("aria-disabled", "true");
  });

  it("Select on non-self rows is enabled", () => {
    renderTable({ rows: [adminUser, generalUser], selfCode: "alice" });
    expect(selectWithLabel("Admin")).toHaveAttribute("aria-disabled", "true");
    // MUI omits `aria-disabled` entirely when the control is enabled
    // (rather than setting it to "false"). Assert "not disabled" via
    // the absence of the explicit true marker.
    expect(selectWithLabel("General").getAttribute("aria-disabled")).not.toBe("true");
  });

  it("dropdown options are admin and general only (no root)", async () => {
    renderTable({ rows: [adminUser] });
    await userEvent.click(selectWithLabel("Admin"));
    expect(screen.getByRole("option", { name: "Admin" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "General" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: "Root" })).not.toBeInTheDocument();
  });

  it("calls onRoleChange when a different role is picked", async () => {
    const onRoleChange = vi.fn();
    renderTable({ rows: [generalUser], onRoleChange });
    await userEvent.click(selectWithLabel("General"));
    await userEvent.click(screen.getByRole("option", { name: "Admin" }));
    expect(onRoleChange).toHaveBeenCalledWith("bob", "admin");
  });

  it("disables every Select while mutationLoading is true", () => {
    renderTable({ rows: [adminUser, generalUser], mutationLoading: true });
    const selects = screen.getAllByRole("combobox");
    expect(selects.every((s) => s.getAttribute("aria-disabled") === "true")).toBe(true);
  });
});

describe("UserTable — self-disable", () => {
  it("disables the Switch on the row whose code matches selfCode", () => {
    renderTable({ rows: [adminUser], selfCode: "alice" });
    const sw = getSwitches()[0];
    expect(sw.disabled).toBe(true);
  });

  it("does NOT disable the Switch on other rows", () => {
    renderTable({ rows: [adminUser, generalUser], selfCode: "alice" });
    const switches = getSwitches();
    expect(switches[0].disabled).toBe(true); // alice
    expect(switches[1].disabled).toBe(false); // bob
  });
});

describe("UserTable — mutation loading", () => {
  it("disables every Switch while mutationLoading is true", () => {
    renderTable({ rows: [adminUser, generalUser], mutationLoading: true });
    const switches = getSwitches();
    expect(switches.every((s) => s.disabled)).toBe(true);
  });
});

describe("UserTable — toggle interaction", () => {
  it("calls onToggle with the row's code and the new checked value", async () => {
    const onToggle = vi.fn();
    renderTable({ rows: [generalUser], onToggle });
    const sw = getSwitches()[0];
    await userEvent.click(sw);
    expect(onToggle).toHaveBeenCalledWith("bob", true);
  });
});