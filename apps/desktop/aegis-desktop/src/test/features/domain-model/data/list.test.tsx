import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { mockCommands, mockInvoke } from "../../../helpers/tauri-mock";
import { TestQueryProvider } from "../../../helpers/test-query-provider";
import {
  useCreateSdtmVariable,
  useDeleteSdtmVariable,
  useGetSdtmDomain,
  useListSdtmVariables,
  useUpdateSdtmDomain,
  useUpdateSdtmVariable,
} from "../../../../features/domain-model/data/list";

function Probe(props: {
  domainId: number | null;
  versionId: number;
  onResult?: (result: unknown) => void;
}) {
  const domain = useGetSdtmDomain(props.domainId);
  const vars = useListSdtmVariables(props.domainId ?? 0);
  const updateDomain = useUpdateSdtmDomain();
  const createVar = useCreateSdtmVariable();
  const updateVar = useUpdateSdtmVariable();
  const deleteVar = useDeleteSdtmVariable();
  return (
    <div>
      <span data-testid="domain-status">{domain.status}</span>
      <span data-testid="vars-status">{vars.status}</span>
      <button
        data-testid="update-domain"
        onClick={() =>
          updateDomain.mutate({ id: props.versionId, body: { name: "AE" } })
        }
      >
        update
      </button>
      <button
        data-testid="create-var"
        onClick={() =>
          createVar.mutate({
            domainId: props.versionId,
            name: "AGE",
            variableType: "Numeric",
            variableCore: "Req",
            variableSequence: 1,
            descriptions: [],
          })
        }
      >
        create
      </button>
      <button
        data-testid="update-var"
        onClick={() =>
          updateVar.mutate({ id: 1, body: { name: "AGE" } })
        }
      >
        update
      </button>
      <button
        data-testid="delete-var"
        onClick={() => deleteVar.mutate(1)}
      >
        delete
      </button>
    </div>
  );
}

const sampleDomain = {
  id: 1,
  versionId: 1,
  name: "AE",
  category: "Events",
  descriptions: [],
  createdAt: "",
  updatedAt: "",
};

const sampleVariables = [
  {
    id: 1,
    domainId: 1,
    name: "AGE",
    variableType: "Numeric",
    variableCore: "Req",
    variableSequence: 1,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
];

beforeEach(() => {
  mockInvoke.mockReset();
  mockCommands({
    get_sdtm_domain_by_id: () => sampleDomain,
    list_sdtm_variables_by_domain: () => ({ variables: sampleVariables }),
    update_sdtm_domain: () => sampleDomain,
    create_sdtm_variable: () => sampleVariables[0],
    update_sdtm_variable: () => sampleVariables[0],
    delete_sdtm_variable: () => undefined,
  });
});

afterEach(() => cleanup());

function renderProbe(domainId: number | null, versionId: number = 1) {
  return render(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <Probe domainId={domainId} versionId={versionId} />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("domain-model data hooks", () => {
  it("useGetSdtmDomain fetches and exposes the domain", async () => {
    renderProbe(1);
    await waitFor(() =>
      expect(screen.getByTestId("domain-status").textContent).toBe("success"),
    );
  });

  it("useGetSdtmDomain stays idle when id is null", async () => {
    renderProbe(null);
    await waitFor(() =>
      expect(screen.getByTestId("domain-status").textContent).toBe("pending"),
    );
  });

  it("useListSdtmVariables fetches variables for the domain", async () => {
    renderProbe(1);
    await waitFor(() =>
      expect(screen.getByTestId("vars-status").textContent).toBe("success"),
    );
  });

  it("useUpdateSdtmDomain calls the API", async () => {
    renderProbe(1);
    screen.getByTestId("update-domain").click();
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      "update_sdtm_domain",
      expect.objectContaining({ id: 1 }),
    ));
  });

  it("useCreateSdtmVariable calls the API", async () => {
    renderProbe(1);
    screen.getByTestId("create-var").click();
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      "create_sdtm_variable",
      expect.objectContaining({
        input: expect.objectContaining({ name: "AGE" }),
      }),
    ));
  });

  it("useUpdateSdtmVariable calls the API", async () => {
    renderProbe(1);
    screen.getByTestId("update-var").click();
    await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith(
      "update_sdtm_variable",
      expect.objectContaining({ id: 1 }),
    ));
  });

  it("useDeleteSdtmVariable calls the API", async () => {
    renderProbe(1);
    screen.getByTestId("delete-var").click();
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "delete_sdtm_variable",
        expect.objectContaining({ id: 1 }),
      ),
    );
  });
});