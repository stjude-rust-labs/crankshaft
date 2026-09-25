<script setup lang="ts">
import { links } from "../../pages";
</script>

<template>
  <section class="sprocket" aria-labelledby="sprocket-title">
    <div class="sprocket__inner">
      <p>
        <b id="sprocket-title">Want to run workflows, not build an engine?</b>
        Sprocket is a WDL workflow engine built on Crankshaft. If you need something that works today, start there.
      </p>
      <a class="sprocket__link" :href="links.sprocket">Go to Sprocket →</a>
    </div>
  </section>
</template>
