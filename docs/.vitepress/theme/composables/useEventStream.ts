import { computed, onBeforeUnmount, onMounted, ref, shallowRef, type Ref } from "vue";

export type Tone = "done" | "problem" | "";

export type StreamEvent = {
  key: number;
  time: string;
  task: string;
  backend: string;
  event: string;
  detail: string;
  tone: Tone;
};

// Synthetic stream: real `crankshaft::events::Event` variants, tasks from
// Whirligig Works, a fictional factory of gizmos, kazoos, and doodads.
const script: Omit<StreamEvent, "key" | "time" | "tone">[] = [
  { task: "polish-gizmo", backend: "docker", event: "TaskCreated", detail: "" },
  { task: "polish-gizmo", backend: "docker", event: "ImagePullStarted", detail: "burnish:3.1" },
  { task: "tune-kazoo", backend: "tes", event: "TaskCreated", detail: "tes_id task-7f3c" },
  { task: "polish-gizmo", backend: "docker", event: "ImagePullFailed", detail: "burnish:3.1" },
  { task: "polish-gizmo", backend: "docker", event: "ImagePullStarted", detail: "burnish:latest" },
  { task: "count-cogs", backend: "lsf", event: "TaskCreated", detail: "" },
  { task: "polish-gizmo", backend: "docker", event: "ImagePullFinished", detail: "burnish:latest" },
  { task: "count-cogs", backend: "lsf", event: "TaskStarted", detail: "job 88213" },
  { task: "polish-gizmo", backend: "docker", event: "TaskContainerCreated", detail: "k3x9q2" },
  { task: "polish-gizmo", backend: "docker", event: "TaskStarted", detail: "" },
  { task: "tune-kazoo", backend: "tes", event: "TaskStarted", detail: "" },
  { task: "polish-gizmo", backend: "docker", event: "TaskStdout", detail: "gizmo 97% shiny" },
  { task: "polish-gizmo", backend: "docker", event: "TaskContainerExited", detail: "exit 0" },
  { task: "polish-gizmo", backend: "docker", event: "TaskCompleted", detail: "exit 0" },
  { task: "tune-kazoo", backend: "tes", event: "TaskPreempted", detail: "spot reclaimed" },
  { task: "count-cogs", backend: "lsf", event: "TaskCompleted", detail: "exit 0" },
  { task: "tune-kazoo", backend: "tes", event: "TaskCreated", detail: "resubmitted" },
  { task: "box-doodads", backend: "docker", event: "TaskCanceled", detail: "token canceled" },
];

export const VISIBLE_ROWS = 7;
const TICK_MS = 2200;
const START = 9 * 3600 + 41 * 60 + 2;

const toneOf = (event: string): Tone =>
  event === "TaskCompleted" || event === "ImagePullFinished"
    ? "done"
    : /Failed|Preempted|Canceled/.test(event)
      ? "problem"
      : "";

// Pure in `index` so server and client render identical initial rows.
function eventAt(index: number): StreamEvent {
  const entry = script[index % script.length];
  const clock = START + index * 5 + ((index * 7) % 5);
  const hh = String(Math.floor(clock / 3600) % 24).padStart(2, "0");
  const mm = String(Math.floor(clock / 60) % 60).padStart(2, "0");
  const ss = String(clock % 60).padStart(2, "0");
  return { ...entry, key: index, time: `${hh}:${mm}:${ss}`, tone: toneOf(entry.event) };
}

const initial = Array.from({ length: VISIBLE_ROWS }, (_, i) => eventAt(VISIBLE_ROWS - 1 - i));

// Module-level state is shared by every subscriber (the hero drawing and the stream table).
const rows = shallowRef<StreamEvent[]>(initial);
const paused = ref(false);
const pageVisible = ref(true);
const visibility = ref(new Map<symbol, boolean>());
const anyVisible = computed(() => [...visibility.value.values()].some(Boolean));
const running = computed(() => !paused.value && pageVisible.value && anyVisible.value);
const latest = computed(() => rows.value[0]);

let next = VISIBLE_ROWS;
let timer: ReturnType<typeof setInterval> | undefined;
let firstTick: ReturnType<typeof setTimeout> | undefined;
let motionQuery: MediaQueryList | undefined;

function tick() {
  if (!running.value) return;
  rows.value = [eventAt(next++), ...rows.value].slice(0, VISIBLE_ROWS);
}

const onMotionChange = (e: MediaQueryListEvent) => {
  if (e.matches) paused.value = true;
};
const onPageVisibility = () => {
  pageVisible.value = document.visibilityState === "visible";
};

function start() {
  motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
  if (motionQuery.matches) paused.value = true;
  motionQuery.addEventListener("change", onMotionChange);
  document.addEventListener("visibilitychange", onPageVisibility);
  onPageVisibility();
  // The first event arrives after half a period so the page comes alive sooner.
  firstTick = setTimeout(() => {
    tick();
    timer = setInterval(tick, TICK_MS);
  }, TICK_MS / 2);
}

function stop() {
  clearTimeout(firstTick);
  clearInterval(timer);
  firstTick = undefined;
  timer = undefined;
  motionQuery?.removeEventListener("change", onMotionChange);
  document.removeEventListener("visibilitychange", onPageVisibility);
}

if (import.meta.hot) import.meta.hot.dispose(stop);

export function useEventStream(target: Ref<Element | undefined>) {
  const id = Symbol("subscriber");
  let observer: IntersectionObserver | undefined;

  const setVisible = (value: boolean) => {
    const map = new Map(visibility.value);
    map.set(id, value);
    visibility.value = map;
  };

  onMounted(() => {
    if (visibility.value.size === 0) start();
    setVisible(false);
    observer = new IntersectionObserver(([entry]) => setVisible(entry.isIntersecting), { threshold: 0.2 });
    if (target.value) observer.observe(target.value);
  });

  onBeforeUnmount(() => {
    observer?.disconnect();
    const map = new Map(visibility.value);
    map.delete(id);
    visibility.value = map;
    if (map.size === 0) stop();
  });

  return {
    rows,
    latest,
    paused,
    running,
    toggle: () => (paused.value = !paused.value),
  };
}
