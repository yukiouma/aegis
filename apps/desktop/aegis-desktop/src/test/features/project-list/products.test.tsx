import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { useListProducts } from "../../../features/project-list";
import type { ProductView } from "../../../shared/api";
import { mockCommands } from "../../../test/helpers/tauri-mock";
import { renderWithQueryClient } from "../../../test/helpers/render-with-query-client";

const productFixture: ProductView = {
  id: 10,
  code: "prod-a",
  name: "Product A",
  description: "Product A description",
  active: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  cleanup();
});

function ProductsProbe() {
  const q = useListProducts();
  return (
    <span data-testid="count">
      {q.data?.length ?? "none"}
    </span>
  );
}

describe("useListProducts", () => {
  it("invokes api.listProducts on mount and exposes the array", async () => {
    mockCommands({ list_products: () => [productFixture] });
    renderWithQueryClient(<ProductsProbe />);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_products");
      expect(screen.getByTestId("count").textContent).toBe("1");
    });
  });

  it("propagates ApiError into query.error", async () => {
    mockCommands({
      list_products: () => {
        throw { kind: "http", status: 500, code: "server", message: "boom" };
      },
    });
    function ErrorProbe() {
      const q = useListProducts();
      return (
        <span data-testid="error-kind">
          {q.error ? (q.error as { kind: string }).kind : "none"}
        </span>
      );
    }
    renderWithQueryClient(<ErrorProbe />);
    await waitFor(() => {
      expect(screen.getByTestId("error-kind").textContent).toBe("http");
    });
  });
});