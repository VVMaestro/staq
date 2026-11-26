import { describe, expect, test } from "bun:test";
import {
  clampSwipeOffset,
  computeSwipeVelocity,
  isSwipePointerType,
  resolveSwipeAxis,
  resolveSwipeOutcome,
} from "../src/taskSwipe";

describe("isSwipePointerType", () => {
  test("accepts touch and pen while ignoring mouse", () => {
    expect(isSwipePointerType("touch")).toBe(true);
    expect(isSwipePointerType("pen")).toBe(true);
    expect(isSwipePointerType("mouse")).toBe(false);
    expect(isSwipePointerType("")).toBe(false);
  });
});

describe("resolveSwipeAxis", () => {
  test("stays pending inside the initial movement dead-zone", () => {
    expect(resolveSwipeAxis(0, 0)).toBe("pending");
    expect(resolveSwipeAxis(6, 5)).toBe("pending");
    expect(resolveSwipeAxis(-9, 0)).toBe("pending");
  });

  test("locks to horizontal when horizontal movement clearly beats vertical", () => {
    expect(resolveSwipeAxis(-20, 4)).toBe("horizontal");
    expect(resolveSwipeAxis(18, -8)).toBe("horizontal");
  });

  test("locks to vertical when scrolling vertically or diagonally", () => {
    expect(resolveSwipeAxis(4, 20)).toBe("vertical");
    expect(resolveSwipeAxis(-12, 12)).toBe("vertical");
    expect(resolveSwipeAxis(14, -14)).toBe("vertical");
  });
});

describe("clampSwipeOffset", () => {
  test("allows left swipe and blocks right swipe for waiting tasks", () => {
    expect(clampSwipeOffset(-80, false, 300)).toBe(-80);
    expect(clampSwipeOffset(-420, false, 300)).toBe(-300);
    expect(clampSwipeOffset(40, false, 300)).toBe(0);
    expect(clampSwipeOffset(200, false, 300)).toBe(0);
  });

  test("allows both left and right swipe for the current task", () => {
    expect(clampSwipeOffset(-90, true, 300)).toBe(-90);
    expect(clampSwipeOffset(90, true, 300)).toBe(90);
    expect(clampSwipeOffset(450, true, 300)).toBe(300);
    expect(clampSwipeOffset(-450, true, 300)).toBe(-300);
  });
});

describe("resolveSwipeOutcome", () => {
  test("triggers cancel when swiped left past threshold on any task", () => {
    expect(resolveSwipeOutcome(-110, 300, -0.1, false)).toBe("cancel");
    expect(resolveSwipeOutcome(-110, 300, -0.1, true)).toBe("cancel");
    expect(resolveSwipeOutcome(-100, 300, -0.1, false)).toBeNull();
  });

  test("triggers complete only when current task is swiped right past threshold", () => {
    expect(resolveSwipeOutcome(110, 300, 0.1, true)).toBe("complete");
    expect(resolveSwipeOutcome(110, 300, 0.1, false)).toBeNull();
    expect(resolveSwipeOutcome(100, 300, 0.1, true)).toBeNull();
  });

  test("triggers on fast flick past minimum travel distance", () => {
    expect(resolveSwipeOutcome(-50, 300, -0.7, false)).toBe("cancel");
    expect(resolveSwipeOutcome(50, 300, 0.7, true)).toBe("complete");
    expect(resolveSwipeOutcome(50, 300, 0.7, false)).toBeNull();
    expect(resolveSwipeOutcome(-20, 300, -1.2, false)).toBeNull();
    expect(resolveSwipeOutcome(20, 300, 1.2, true)).toBeNull();
  });
});

describe("computeSwipeVelocity", () => {
  test("returns zero when finger paused before release", () => {
    const samples = [
      { x: 200, time: 0 },
      { x: 120, time: 40 },
    ];
    expect(computeSwipeVelocity(samples, 160)).toBe(0);
  });

  test("preserves flick velocity even with a duplicate final touch coordinate", () => {
    const samples = [
      { x: 200, time: 100 },
      { x: 130, time: 150 },
      { x: 130, time: 160 },
    ];
    expect(computeSwipeVelocity(samples, 165)).toBeCloseTo(-70 / 60, 4);
  });

  test("computes velocity over the recent window rather than whole gesture", () => {
    const samples = [
      { x: 200, time: 0 },
      { x: 190, time: 200 },
      { x: 140, time: 250 },
    ];
    expect(computeSwipeVelocity(samples, 260)).toBeCloseTo(-1, 4);
  });
});
