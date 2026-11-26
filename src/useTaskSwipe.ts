import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";
import {
  SWIPE_TRIGGER_RATIO,
  clampSwipeOffset,
  computeSwipeVelocity,
  isSwipePointerType,
  resolveSwipeAxis,
  resolveSwipeOutcome,
  type SwipeAction,
  type SwipeAxis,
  type SwipeSample,
} from "./taskSwipe";

interface UseTaskSwipeOptions {
  surfaceRef: Ref<HTMLElement | null>;
  current: () => boolean;
  busy: () => boolean;
  menuOpen: () => boolean;
  closeMenu: () => void;
  onOutcome: (action: SwipeAction) => void;
}

const SNAP_BACK_MS = 180;
const FLY_OUT_MS = 140;
const CLICK_SUPPRESS_MS = 250;
const MAX_SAMPLES = 12;

export function useTaskSwipe(options: UseTaskSwipeOptions) {
  const offset = ref(0);
  const isDragging = ref(false);
  const committing = ref<SwipeAction | null>(null);
  const snapBackAction = ref<SwipeAction | null>(null);
  const trackedCardWidth = ref(300);

  let activePointerId: number | null = null;
  let startX = 0;
  let startY = 0;
  let axis: SwipeAxis = "pending";
  let samples: SwipeSample[] = [];
  let suppressNextClick = false;
  let commitTimer: number | null = null;
  let snapBackTimer: number | null = null;
  let suppressTimer: number | null = null;

  const swipeAction = computed<SwipeAction | null>(() => {
    if (committing.value) return committing.value;
    if (offset.value < 0) return "cancel";
    if (offset.value > 0) return "complete";
    return snapBackAction.value;
  });

  const swipeReady = computed(
    () =>
      committing.value !== null ||
      Math.abs(offset.value) >= trackedCardWidth.value * SWIPE_TRIGGER_RATIO,
  );

  const surfaceStyle = computed(() =>
    offset.value === 0 ? undefined : { transform: `translateX(${offset.value}px)` },
  );

  function clearSnapBackTimer() {
    if (snapBackTimer !== null) {
      window.clearTimeout(snapBackTimer);
      snapBackTimer = null;
    }
  }

  function snapBack(fromAction: SwipeAction | null) {
    committing.value = null;
    offset.value = 0;
    clearSnapBackTimer();

    if (!fromAction) {
      snapBackAction.value = null;
      return;
    }

    // Keep the backdrop mounted until the 180ms return transition finishes.
    snapBackAction.value = fromAction;
    snapBackTimer = window.setTimeout(() => {
      snapBackTimer = null;
      snapBackAction.value = null;
    }, SNAP_BACK_MS);
  }

  watch(options.busy, (busy, wasBusy) => {
    if (!busy && wasBusy && committing.value !== null) {
      snapBack(committing.value);
    }
  });

  function resetTracking() {
    activePointerId = null;
    axis = "pending";
    samples = [];
  }

  function recordSample(x: number, time: number) {
    samples.push({ x, time });
    if (samples.length > MAX_SAMPLES) {
      samples.shift();
    }
  }

  function consumeSuppressedClick(): boolean {
    if (!suppressNextClick) return false;
    suppressNextClick = false;
    if (suppressTimer !== null) {
      window.clearTimeout(suppressTimer);
      suppressTimer = null;
    }
    return true;
  }

  function handlePointerDown(event: PointerEvent) {
    if (!isSwipePointerType(event.pointerType)) return;
    if (options.busy() || committing.value !== null) return;
    if (event.target instanceof Element && event.target.closest("a, button")) return;
    if (options.menuOpen()) {
      options.closeMenu();
      return;
    }

    clearSnapBackTimer();
    snapBackAction.value = null;

    activePointerId = event.pointerId;
    startX = event.clientX;
    startY = event.clientY;
    axis = "pending";
    samples = [{ x: event.clientX, time: event.timeStamp }];
    trackedCardWidth.value = options.surfaceRef.value?.offsetWidth || 300;
  }

  function handlePointerMove(event: PointerEvent) {
    if (event.pointerId !== activePointerId) return;

    const dx = event.clientX - startX;
    const dy = event.clientY - startY;

    if (axis === "pending") {
      const nextAxis = resolveSwipeAxis(dx, dy);
      if (nextAxis === "pending") return;
      if (nextAxis === "vertical") {
        resetTracking();
        return;
      }

      // Waiting tasks cannot be swiped right; ignore rightward drags completely.
      if (!options.current() && dx > 0) {
        return;
      }

      axis = "horizontal";
      isDragging.value = true;
      try {
        options.surfaceRef.value?.setPointerCapture(event.pointerId);
      } catch {
        // Ignore pointer capture errors on synthetic or cancelled pointers.
      }
    }

    if (axis === "horizontal") {
      recordSample(event.clientX, event.timeStamp);
      offset.value = clampSwipeOffset(dx, options.current(), trackedCardWidth.value);
    }
  }

  function commitOutcome(outcome: SwipeAction) {
    committing.value = outcome;
    offset.value =
      outcome === "cancel" ? -(trackedCardWidth.value + 24) : trackedCardWidth.value + 24;

    const prefersReducedMotion =
      typeof window !== "undefined" &&
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const delayMs = prefersReducedMotion ? 0 : FLY_OUT_MS;

    if (commitTimer !== null) window.clearTimeout(commitTimer);
    commitTimer = window.setTimeout(() => {
      commitTimer = null;
      if (options.busy()) {
        snapBack(outcome);
        return;
      }
      options.onOutcome(outcome);
      // Recover even if the parent handler returns early or throws synchronously without flipping busy.
      void nextTick(() => {
        if (!options.busy() && committing.value === outcome) {
          snapBack(outcome);
        }
      });
    }, delayMs);
  }

  function handlePointerUp(event: PointerEvent) {
    if (event.pointerId !== activePointerId) return;

    const wasHorizontal = axis === "horizontal";
    const currentOffset = offset.value;
    const velocityX = wasHorizontal ? computeSwipeVelocity(samples, event.timeStamp) : 0;
    resetTracking();

    if (!wasHorizontal) return;

    try {
      if (options.surfaceRef.value?.hasPointerCapture(event.pointerId)) {
        options.surfaceRef.value.releasePointerCapture(event.pointerId);
      }
    } catch {
      // Ignore if capture was already released by the browser.
    }

    isDragging.value = false;
    suppressNextClick = true;
    if (suppressTimer !== null) window.clearTimeout(suppressTimer);
    suppressTimer = window.setTimeout(() => {
      suppressTimer = null;
      suppressNextClick = false;
    }, CLICK_SUPPRESS_MS);

    const outcome = resolveSwipeOutcome(
      currentOffset,
      trackedCardWidth.value,
      velocityX,
      options.current(),
    );
    if (outcome) {
      commitOutcome(outcome);
    } else {
      const priorAction: SwipeAction | null =
        currentOffset < 0 ? "cancel" : currentOffset > 0 ? "complete" : null;
      snapBack(priorAction);
    }
  }

  function handlePointerCancel(event: PointerEvent) {
    if (event.pointerId !== activePointerId) return;
    const priorAction: SwipeAction | null =
      offset.value < 0 ? "cancel" : offset.value > 0 ? "complete" : null;
    resetTracking();
    isDragging.value = false;
    snapBack(priorAction);
  }

  onBeforeUnmount(() => {
    if (commitTimer !== null) window.clearTimeout(commitTimer);
    clearSnapBackTimer();
    if (suppressTimer !== null) window.clearTimeout(suppressTimer);
  });

  return {
    isDragging,
    swipeAction,
    swipeReady,
    surfaceStyle,
    consumeSuppressedClick,
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
    handlePointerCancel,
  };
}
