import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { mockCommands, mockInvoke } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { useSdtmContext } from "./sdtm";
import type {
  ProjectView,
  SdtmDomainView,
  SdtmVersionView,
} from "../../../shared/api";

function Probe({ projectCode }: { projectCode: string }) {
  const ctx = useSdtmContext(projectCode);
  return (
    <pre data-testid="ctx">
      {JSON.stringify({
        versionId: ctx.versionId,
        language: ctx.language,
        domainsLen: ctx.domains.length,
        loading: ctx.loading,
        error: ctx.error ? (ctx.error as any).message ?? null : null,
      })}
    </pre>
  );
}

function mount(projectCode: string) {
  return render(
    <TestQueryProvider>
      <AegisI18nProvider>
        <Probe projectCode={projectCode} />
      </AegisI18nProvider>
    </TestQueryProvider>,
  );
}

const versions: SdtmVersionView[] = [
  { id: 1, name: "SDTMIG v3.3", createdAt: "", updatedAt: "" },
  { id: 5, name: "SDTMIG v3.4", createdAt: "", updatedAt: "" },
  { id: 3, name: "SDTMIG v3.2", createdAt: "", updatedAt: "" },
];

const domains: SdtmDomainView[] = [
  {
    id: 100,
    versionId: 5,
    name: "AE",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Adverse Events", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
];

const baseProject: ProjectView = {
  id: 1,
  code: "abc",
  description: "",
  members: { leaders: [], workers: [] },
  unblindMembers: { leaders: [], workers: [] },
  configurations: { language: null, tags: [] },
  active: true,
  createdAt: "",
  updatedAt: "",
};

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => cleanup());

describe("useSdtmContext — versionId", () => {
  it("uses the highest id when sdtmig is absent", async () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(5),
    );
  });

  it("uses the configured versionId when it is valid", async () => {
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: null,
          tags: [],
          sdtmig: { versionId: 3, versionName: "SDTMIG v3.2" },
        },
      }),
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(3),
    );
  });

  it("self-heals when versionId is stale but versionName matches", async () => {
    let updateCalls = 0;
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: "en",
          tags: [],
          sdtmig: { versionId: 999, versionName: "SDTMIG v3.4" },
        },
      }),
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
      update_project: () => {
        updateCalls += 1;
        return baseProject;
      },
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(5),
    );
    // Give the microtask queue a chance to flush the mutation.
    await waitFor(() => expect(updateCalls).toBeGreaterThanOrEqual(1));
    // Re-render does not fire a second correction.
    await new Promise((r) => setTimeout(r, 50));
    expect(updateCalls).toBe(1);
  });

  it("falls back to highest id when both versionId and versionName are stale", async () => {
    let updateCalls = 0;
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: null,
          tags: [],
          sdtmig: { versionId: 999, versionName: "ghost" },
        },
      }),
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
      update_project: () => {
        updateCalls += 1;
        return baseProject;
      },
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(5),
    );
    await new Promise((r) => setTimeout(r, 50));
    expect(updateCalls).toBe(0);
  });

  it("preserves language + tags in the recovery mutation body", async () => {
    let captured: any = null;
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: "zh-CN",
          tags: [{ key: "k", value: "v" }],
          sdtmig: { versionId: 999, versionName: "SDTMIG v3.4" },
        },
      }),
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
      update_project: (args: any) => {
        captured = args;
        return baseProject;
      },
    });
    mount("abc");
    await waitFor(() => expect(captured).not.toBeNull());
    expect(captured.body.configurations).toEqual({
      language: "zh-CN",
      tags: [{ key: "k", value: "v" }],
      sdtmig: { versionId: 5, versionName: "SDTMIG v3.4" },
    });
  });
});

describe("useSdtmContext — language", () => {
  it("returns project.language when set", async () => {
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: { language: "zh-CN", tags: [] },
      }),
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).language).toBe("zh-CN"),
    );
  });

  it("falls back to 'en' when project.language is null", async () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      list_sdtm_versions: () => ({ versions }),
      list_sdtm_domains_by_version: () => ({ domains }),
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).language).toBe("en"),
    );
  });
});

describe("useSdtmContext — loading + error", () => {
  it("is loading while versions are still pending", () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      // No list_sdtm_versions handler — versionsQuery stays pending.
    });
    mount("abc");
    const parsed = JSON.parse(screen.getByTestId("ctx").textContent!);
    expect(parsed.loading).toBe(true);
    expect(parsed.versionId).toBeNull();
  });

  it("surfaces an error from the versions query", async () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      list_sdtm_versions: () => {
        throw { kind: "http", status: 500, code: "internal", message: "boom" };
      },
      list_sdtm_domains_by_version: () => ({ domains }),
    });
    mount("abc");
    await waitFor(() => {
      const parsed = JSON.parse(screen.getByTestId("ctx").textContent!);
      expect(parsed.error).toContain("boom");
    });
  });
});