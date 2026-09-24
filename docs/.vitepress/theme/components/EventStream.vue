<script setup lang="ts">
import { ref } from "vue";
import { withBase } from "vitepress";
import { useEventStream } from "../composables/useEventStream";

const root = ref<HTMLElement>();
const { rows, paused, toggle } = useEventStream(root);
</script>

<template>
  <section ref="root" class="home-block stream" aria-labelledby="stream-title">
    <div class="home-block__inner stream__inner">
      <div class="stream__intro">
        <h2 id="stream-title" class="home-block__title">Every task reports in</h2>
        <p class="home-block__intro">
          Subscribe to the engine and each task announces itself as it moves: created, image pulled, started, and
          finished. The same stream feeds the gRPC monitor and the terminal console.
        </p>
        <p class="stream__links">
          <a :href="withBase('/observe/events')">Events</a> · <a :href="withBase('/observe/monitoring')">Monitoring</a>
        </p>
      </div>

      <div class="stream__panel">
        <div class="stream__bar">
          <code class="stream__call">engine.subscribe()</code>
          <span class="stream__note">Example output</span>
          <button
            type="button"
            class="stream__toggle"
            :aria-pressed="paused"
            :aria-label="paused ? 'Resume example event stream' : 'Pause example event stream'"
            @click="toggle"
          >
            <svg v-if="paused" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5.5v13l10-6.5z" /></svg>
            <svg v-else viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>
            {{ paused ? "Resume" : "Pause" }}
          </button>
        </div>
        <table class="stream__table">
          <caption class="visually-hidden">Example Crankshaft event stream, newest first</caption>
          <thead>
            <tr>
              <th scope="col"><span class="visually-hidden">Kind</span></th>
              <th scope="col">Time</th>
              <th scope="col">Event</th>
              <th scope="col" class="stream__col-task">Task</th>
              <th scope="col">Backend</th>
              <th scope="col" class="stream__col-detail">Detail</th>
            </tr>
          </thead>
          <TransitionGroup tag="tbody" name="stream-row">
            <tr v-for="row in rows" :key="row.key" :data-tone="row.tone">
              <td class="stream__mark"><span /></td>
              <td class="stream__time">{{ row.time }}</td>
              <td class="stream__event">{{ row.event }}</td>
              <td class="stream__col-task">{{ row.task }}</td>
              <td class="stream__backend">{{ row.backend }}</td>
              <td class="stream__col-detail">{{ row.detail }}</td>
            </tr>
          </TransitionGroup>
        </table>
      </div>
    </div>
  </section>
</template>
