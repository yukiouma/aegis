import type {
  ProjectConfiguration,
  ProjectView,
} from "../../shared/api";

/**
 * Build a `ProjectView` with the new `configurations: { language, tags }`
 * shape, pre-filled with sensible defaults so individual tests only have
 * to override the fields they care about. Centralised here so every
 * project-shaped fixture across the test suite stays in lockstep with
 * the wire shape — if the wire ever changes again, this is the one place
 * to update.
 */
export function makeProject(
  overrides: Partial<ProjectView> = {},
): ProjectView {
  const baseConfig: ProjectConfiguration = { language: null, tags: [] };
  return {
    id: 1,
    code: "alpha",
    description: "Alpha description",
    members: { leaders: [], workers: [] },
    unblindMembers: { leaders: [], workers: [] },
    configurations: baseConfig,
    active: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}