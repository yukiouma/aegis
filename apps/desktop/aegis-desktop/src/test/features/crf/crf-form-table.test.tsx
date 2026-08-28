import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";

import { applyReorder, computeReorder } from "../../../features/crf/components/CrfFormTable";

describe("computeReorder", () => {
  afterEach(() => {
    // no-op; placeholder for future shared teardown
  });

  it("moves the source row forward to the target's slot, shifting the target right", () => {
    expect(computeReorder([1, 2, 3, 4], 1, 3)).toEqual([2, 3, 1, 4]);
  });

  it("moves the source row backward to the target's slot, pushing the target right", () => {
    expect(computeReorder([1, 2, 3, 4], 4, 2)).toEqual([1, 4, 2, 3]);
  });

  it("drops to the end of the list when the target is the last row", () => {
    expect(computeReorder([1, 2, 3], 1, 3)).toEqual([2, 3, 1]);
  });

  it("drops to the front of the list when the target is the first row", () => {
    expect(computeReorder([1, 2, 3], 3, 1)).toEqual([3, 1, 2]);
  });

  it("returns null when source equals target (no-op drop on self)", () => {
    expect(computeReorder([1, 2, 3], 2, 2)).toBeNull();
  });

  it("returns null when the source id is not in the list", () => {
    expect(computeReorder([1, 2, 3], 99, 1)).toBeNull();
  });

  it("returns null when the target id is not in the list", () => {
    expect(computeReorder([1, 2, 3], 1, 99)).toBeNull();
  });

  it("does not mutate the input array", () => {
    const input = [1, 2, 3, 4];
    computeReorder(input, 1, 3);
    expect(input).toEqual([1, 2, 3, 4]);
  });
});

describe("applyReorder", () => {
  const event = (
    sourceId: string | number | null,
    targetId: string | number | null,
    canceled = false,
  ) => ({
    canceled,
    operation: {
      source: sourceId == null ? null : { id: sourceId },
      target: targetId == null ? null : { id: targetId },
    },
  });

  it("reads source.id (the dragged row) — moves the source to the target's slot", () => {
    expect(applyReorder([1, 2, 3], event("1", "3"))).toEqual([2, 3, 1]);
    expect(applyReorder([1, 2, 3], event("3", "1"))).toEqual([3, 1, 2]);
  });

  it("returns null when the drag was canceled", () => {
    expect(applyReorder([1, 2, 3], event("1", "3", true))).toBeNull();
  });

  it("returns null when source is missing (drop outside any draggable)", () => {
    expect(applyReorder([1, 2, 3], event(null, "1"))).toBeNull();
  });

  it("returns null when target is missing (drop outside any droppable)", () => {
    expect(applyReorder([1, 2, 3], event("1", null))).toBeNull();
  });

  it("returns null when source equals target", () => {
    expect(applyReorder([1, 2, 3], event("2", "2"))).toBeNull();
  });

  it("coerces string ids to numbers before indexing", () => {
    expect(applyReorder([1, 2, 3, 4], event("1", "3"))).toEqual([2, 3, 1, 4]);
  });

  it("returns null when either id fails to coerce to a finite number", () => {
    expect(applyReorder([1, 2, 3], event("abc", "1"))).toBeNull();
  });
});
