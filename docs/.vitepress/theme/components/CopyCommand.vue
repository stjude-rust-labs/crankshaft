<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{ command: string }>();
const status = ref<"idle" | "copied" | "failed">("idle");
let timer: ReturnType<typeof setTimeout> | undefined;

async function copy() {
  try {
    await navigator.clipboard.writeText(props.command);
    status.value = "copied";
  } catch {
    status.value = "failed";
  }
  clearTimeout(timer);
  timer = setTimeout(() => (status.value = "idle"), 2400);
}
</script>

<template>
  <div class="copy-command" :data-status="status">
    <code class="copy-command__text"><span class="copy-command__prompt" aria-hidden="true">$</span>{{ command }}</code>
    <button type="button" class="copy-command__button" :aria-label="`Copy ${command}`" @click="copy">
      <svg v-if="status === 'copied'" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
      <svg v-else viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="1.5" /><path d="M16 8V5.5A1.5 1.5 0 0 0 14.5 4h-9A1.5 1.5 0 0 0 4 5.5v9A1.5 1.5 0 0 0 5.5 16H8" /></svg>
      <span>{{ status === "copied" ? "Copied" : status === "failed" ? "Copy failed" : "Copy" }}</span>
    </button>
    <span class="visually-hidden" aria-live="polite">{{
      status === "copied" ? "Command copied to clipboard" : status === "failed" ? "Could not copy. Select the command and copy it manually." : ""
    }}</span>
  </div>
</template>
