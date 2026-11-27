export type SwipeAction = "cancel" | "complete";
export type SwipeAxis = "pending" | "horizontal" | "vertical";

export interface SwipeSample {
  x: number;
  time: number;
}

export const SWIPE_LOCK_DISTANCE = 10;
export const SWIPE_TRIGGER_RATIO = 0.35;
export const SWIPE_FLICK_VELOCITY = 0.55;
export const SWIPE_FLICK_MIN_DISTANCE = 44;

const HORIZONTAL_DOMINANCE_RATIO = 1.15;
const VELOCITY_WINDOW_MS = 100;
const VELOCITY_STALE_MS = 80;

export function isSwipePointerType(pointerType: string): boolean {
  return pointerType === "touch" || pointerType === "pen";
}

export function resolveSwipeAxis(dx: number, dy: number): SwipeAxis {
  if (Math.hypot(dx, dy) < SWIPE_LOCK_DISTANCE) {
    return "pending";
  }

  return Math.abs(dx) > Math.abs(dy) * HORIZONTAL_DOMINANCE_RATIO ? "horizontal" : "vertical";
}

export function clampSwipeOffset(dx: number, allowComplete: boolean, maxTravel: number): number {
  const limit = Math.max(0, maxTravel);
  if (!allowComplete && dx > 0) {
    return 0;
  }

  return Math.max(-limit, Math.min(allowComplete ? limit : 0, dx));
}

export function computeSwipeVelocity(samples: readonly SwipeSample[], releaseTime: number): number {
  if (samples.length < 2) return 0;

  const latest = samples[samples.length - 1];
  if (releaseTime - latest.time > VELOCITY_STALE_MS) {
    return 0;
  }

  const windowStart = releaseTime - VELOCITY_WINDOW_MS;
  let anchor = samples[samples.length - 2];
  for (let i = samples.length - 2; i >= 0; i -= 1) {
    if (samples[i].time >= windowStart) {
      anchor = samples[i];
    } else {
      break;
    }
  }

  const dt = latest.time - anchor.time;
  if (dt <= 0) return 0;

  return (latest.x - anchor.x) / dt;
}

export function resolveSwipeOutcome(
  offsetX: number,
  cardWidth: number,
  velocityX: number,
  allowComplete: boolean,
): SwipeAction | null {
  if (cardWidth <= 0 || offsetX === 0) return null;

  const action: SwipeAction | null =
    offsetX < 0 ? "cancel" : allowComplete ? "complete" : null;
  if (!action) return null;

  const sign = action === "cancel" ? -1 : 1;
  const distance = offsetX * sign;
  const directionalVelocity = velocityX * sign;

  if (
    distance >= cardWidth * SWIPE_TRIGGER_RATIO ||
    (distance >= SWIPE_FLICK_MIN_DISTANCE && directionalVelocity >= SWIPE_FLICK_VELOCITY)
  ) {
    return action;
  }

  return null;
}
