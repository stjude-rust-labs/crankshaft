<script setup lang="ts">
import { withBase } from "vitepress";
import { backends } from "../../pages";
</script>

<template>
  <section class="home-block" aria-labelledby="backends-title">
    <div class="home-block__inner">
      <h2 id="backends-title" class="home-block__title">Backends</h2>
      <p class="home-block__intro">
        Register as many as you need, each under its own name, and pick one when you spawn.
      </p>
      <div class="bom">
        <article v-for="b in backends" :key="b.key" class="bom__item">
          <h3 class="bom__name">
            <a :href="withBase(b.link)">{{ b.title }}</a>
          </h3>
          <p class="bom__summary">{{ b.summary }}</p>
          <dl class="bom__specs">
            <dt>Driven by</dt>
            <dd>{{ b.drivenBy }}</dd>
            <dt>Container images</dt>
            <dd>{{ b.images }}</dd>
          </dl>
          <a class="bom__go" :href="withBase(b.link)">{{ b.title }} backend →</a>
        </article>
      </div>
    </div>
  </section>
</template>
