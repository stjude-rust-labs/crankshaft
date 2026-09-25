<script setup lang="ts">
import { onMounted, ref } from "vue";
import { data as servers, type TesServer } from "../../data/tes-servers.data";

const CACHE_KEY = "crankshaft:tes-server-stars";
const CACHE_MS = 60 * 60 * 1000;

// Live counts from the visitor's browser; the build-time counts are the fallback.
const live = ref<Record<string, number>>({});

const starsOf = (s: TesServer) => live.value[s.repo] ?? s.stars;
const format = (n: number) => (n < 1000 ? String(n) : `${(n / 1000).toFixed(1).replace(/\.0$/, "")}k`);

onMounted(async () => {
  try {
    const cached = JSON.parse(sessionStorage.getItem(CACHE_KEY) ?? "null");
    if (cached && Date.now() - cached.at < CACHE_MS) {
      live.value = cached.stars;
      return;
    }
  } catch {
    // Ignore unreadable storage and fetch fresh counts.
  }

  const results = await Promise.all(
    servers.map(async (s) => {
      try {
        const res = await fetch(`https://api.github.com/repos/${s.repo}`, {
          headers: { Accept: "application/vnd.github+json" },
        });
        if (!res.ok) return null;
        const json = (await res.json()) as { stargazers_count?: number };
        return typeof json.stargazers_count === "number" ? ([s.repo, json.stargazers_count] as const) : null;
      } catch {
        return null;
      }
    }),
  );

  const stars = Object.fromEntries(results.filter((r) => r !== null));
  if (Object.keys(stars).length === 0) return;
  live.value = stars;
  try {
    sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), stars }));
  } catch {
    // Storage can be full or disabled; the counts still show.
  }
});
</script>

<template>
  <table class="tes-servers">
    <thead>
      <tr>
        <th>Server</th>
        <th>Where it runs</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="s in servers" :key="s.repo">
        <td>
          <a :href="`https://github.com/${s.repo}`">{{ s.name }}</a>
          <a
            v-if="starsOf(s) !== null"
            class="tes-servers__stars"
            :href="`https://github.com/${s.repo}/stargazers`"
            :aria-label="`${starsOf(s)} stars on GitHub`"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path
                d="M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.751.751 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z"
              />
            </svg>
            {{ format(starsOf(s)!) }}
          </a>
          <span v-if="s.note" class="tes-servers__note">{{ s.note }}</span>
        </td>
        <td>{{ s.runsOn }}</td>
      </tr>
    </tbody>
  </table>
</template>
