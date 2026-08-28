import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";

import { computeReorder } from "../../../features/crf/components/CrfFormTable";

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
