<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { withBase } from "vitepress";
import { backends, links } from "../../pages";
import { useEventStream } from "../composables/useEventStream";
import CopyCommand from "./CopyCommand.vue";

// Side-view geometry. Each throw is a pair of webs carrying a pin that
// orbits the centerline; `up` is true when the pin starts above it.
const AXIS = 170;
const PIN_RADIUS = 56;
const PIN_HALF = 20;
const WEB_REACH = 86;
const WEB_TAIL = 22;
const WEB_HALF_WIDTH = 22;

type Throw = {
  webs: [number, number];
  pinX: number;
  up: boolean;
  leadTail: string;
  balloon: [number, number];
  label: [number, number];
};

const throws: Record<number, Throw> = {
  1: { webs: [74, 148], pinX: 96, up: true, leadTail: "L122 58 L150 34 H196", balloon: [150, 34], label: [172, 30] },
  2: { webs: [222, 296], pinX: 244, up: false, leadTail: "L270 280 L296 300 H330", balloon: [296, 300], label: [318, 296] },
  3: { webs: [370, 444], pinX: 392, up: true, leadTail: "L418 58 L440 34 H486", balloon: [440, 34], label: [462, 30] },
};

const figure = ref<HTMLElement>();
const { latest, paused, running, toggle } = useEventStream(figure);
const active = computed(() => backends.find((b) => b.key === latest.value.backend)?.item);

// The crank turns half a revolution per event, then rests until the next one.
const angle = ref(0);
let target = 0;
let frame = 0;
let reduceMotion: MediaQueryList | undefined;

const easeInOut = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);

function turn() {
  cancelAnimationFrame(frame);
  const from = angle.value;
  target += Math.PI;
  const began = performance.now();
  const step = (now: number) => {
    const t = Math.min((now - began) / 1100, 1);
    angle.value = from + (target - from) * easeInOut(t);
    if (t < 1) frame = requestAnimationFrame(step);
  };
  frame = requestAnimationFrame(step);
}

onMounted(() => {
  reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
});

watch(
  () => latest.value.key,
  () => {
    if (running.value && !reduceMotion?.matches) turn();
  },
);

onBeforeUnmount(() => cancelAnimationFrame(frame));

const round = (n: number) => Math.round(n * 100) / 100;

const pose = computed(() => {
  const out: Record<number, { webY: number; webH: number; pinY: number; lead: string }> = {};
  for (const [item, t] of Object.entries(throws)) {
    const phase = angle.value + (t.up ? 0 : Math.PI);
    const c = Math.cos(phase);
    const s = Math.abs(Math.sin(phase));
    const top = Math.max(WEB_REACH * c, -WEB_TAIL * c) + WEB_HALF_WIDTH * s;
    const bottom = Math.min(WEB_REACH * c, -WEB_TAIL * c) - WEB_HALF_WIDTH * s;
    const pinY = AXIS - PIN_RADIUS * c - PIN_HALF;
    const leadX = t.pinX + 26;
    const leadY = t.up ? pinY : pinY + PIN_HALF * 2;
    out[+item] = {
      webY: round(AXIS - top),
      webH: round(top - bottom),
      pinY: round(pinY),
      lead: `M${leadX} ${round(leadY)} ${t.leadTail}`,
    };
  }
  return out;
});
</script>

<template>
  <section class="hero" aria-labelledby="hero-title">
    <div class="hero__copy">
      <h1 id="hero-title" class="hero__title">Task execution for workflow engines</h1>
      <p class="hero__lede">
        Crankshaft is a headless Rust library for running tasks. Your engine builds the tasks, and Crankshaft submits,
        monitors, and cancels them on Docker, GA4GH TES, or any scheduler that can be run from a shell. It was
        developed at St. Jude for the bioinformatics execution engine, <a :href="links.sprocket">Sprocket</a>, and is designed to manage tens to hundreds of thousands of
        concurrent tasks, but it can be used for any kind of work.
      </p>
      <div class="hero__actions">
        <a class="button" :href="withBase('/guide/getting-started')">Get started <span aria-hidden="true">→</span></a>
        <CopyCommand command="cargo add crankshaft" />
      </div>
      <p class="hero__meta">
        Part of the <a :href="links.sprocket">Sprocket</a> project by St. Jude Rust Labs ·
        <a :href="links.docsrs">API reference on docs.rs</a>
      </p>
    </div>

    <figure ref="figure" class="drawing">
      <svg viewBox="0 0 610 330" role="group" aria-label="Drawing with three numbered throws, one for each backend">
        <line class="drawing__cl" x1="8" y1="170" x2="600" y2="170" />
        <g class="drawing__part">
          <rect x="30" y="152" width="44" height="36" />
          <rect x="170" y="148" width="52" height="44" />
          <rect x="318" y="148" width="52" height="44" />
          <rect x="466" y="152" width="40" height="36" />
          <template v-for="(t, item) in throws" :key="item">
            <rect
              v-for="x in t.webs"
              :key="x"
              :x="x"
              :y="pose[item].webY"
              width="22"
              :height="pose[item].webH"
              rx="6"
            />
          </template>
          <rect x="506" y="126" width="16" height="88" rx="2" />
        </g>
        <a
          v-for="b in backends"
          :key="b.item"
          class="drawing__throw"
          :class="{ 'is-active': active === b.item }"
          :href="withBase(b.link)"
          :aria-label="`${b.title} backend: ${b.callout}`"
        >
          <rect class="drawing__pin" :x="throws[b.item].pinX" :y="pose[b.item].pinY" width="52" height="40" />
          <path class="drawing__lead" :d="pose[b.item].lead" />
          <circle
            v-if="active === b.item"
            :key="latest.key"
            class="drawing__pulse"
            :cx="throws[b.item].balloon[0]"
            :cy="throws[b.item].balloon[1]"
            r="13"
          />
          <circle class="drawing__balloon" :cx="throws[b.item].balloon[0]" :cy="throws[b.item].balloon[1]" r="13" />
          <text class="drawing__num" :x="throws[b.item].balloon[0]" :y="throws[b.item].balloon[1]">{{ b.item }}</text>
          <text class="drawing__label" :x="throws[b.item].label[0]" :y="throws[b.item].label[1]">{{ b.title }}</text>
          <text class="drawing__sub" :x="throws[b.item].label[0]" :y="throws[b.item].label[1] + 17">{{ b.callout }}</text>
        </a>
        <text class="drawing__note" x="30" y="292">Shared core: Engine and Task.</text>
        <text class="drawing__note" x="30" y="308">Each throw: a named backend.</text>
      </svg>
      <figcaption class="drawing__caption">
        <span>Every backend accepts the same Task.</span>
        <span class="drawing__live" aria-hidden="true">
          <span :key="latest.key" class="drawing__live-text">{{ latest.event }} · {{ latest.task }} on {{ latest.backend }}</span>
        </span>
        <button
          type="button"
          class="stream__toggle drawing__toggle"
          :aria-pressed="paused"
          :aria-label="paused ? 'Resume the animated drawing and example events' : 'Pause the animated drawing and example events'"
          @click="toggle"
        >
          <svg v-if="paused" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5.5v13l10-6.5z" /></svg>
          <svg v-else viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>
          {{ paused ? "Resume" : "Pause" }}
        </button>
      </figcaption>
    </figure>
  </section>
</template>
