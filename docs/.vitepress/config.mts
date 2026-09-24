import { defineConfig, type DefaultTheme } from "vitepress";
import { links, pages, sections } from "./pages";
import { redlineDark, redlineLight } from "./redline-theme";

const sidebar: DefaultTheme.SidebarItem[] = sections.map((section) => ({
  text: section,
  items: pages.filter((p) => p.section === section).map((p) => ({ text: p.title, link: p.link })),
}));

export default defineConfig({
  title: "Crankshaft",
  description:
    "Crankshaft is a headless task execution library for Rust that runs the same task on Docker, GA4GH TES, or HPC schedulers.",
  base: "/crankshaft/",
  cleanUrls: true,
  lang: "en-US",
  appearance: true,
  lastUpdated: true,
  head: [
    ["link", { rel: "icon", type: "image/svg+xml", href: "/crankshaft/favicon.svg" }],
    ["meta", { name: "theme-color", content: "#D11947" }],
  ],
  markdown: {
    theme: { light: redlineLight, dark: redlineDark },
  },
  themeConfig: {
    logo: { src: "/stjude-logo.svg", alt: "St. Jude: Crankshaft home" },
    siteTitle: false,
    nav: [
      { text: "Get started", link: "/guide/introduction", activeMatch: "^/guide/" },
      { text: "Concepts", link: "/concepts/engine", activeMatch: "^/concepts/" },
      { text: "Backends", link: "/backends/docker", activeMatch: "^/backends/" },
      { text: "Events", link: "/observe/events", activeMatch: "^/observe/" },
      { text: "API reference", link: links.docsrs },
    ],
    sidebar: {
      "/guide/": sidebar,
      "/concepts/": sidebar,
      "/backends/": sidebar,
      "/observe/": sidebar,
    },
    outline: { level: [2, 3], label: "On this page" },
    docFooter: { prev: "Previous", next: "Next" },
    socialLinks: [{ icon: "github", link: links.github, ariaLabel: "Crankshaft on GitHub" }],
    editLink: {
      pattern: "https://github.com/stjude-rust-labs/crankshaft/edit/main/docs/:path",
      text: "Suggest a change to this page",
    },
    search: { provider: "local" },
    footer: {
      message:
        'Licensed MIT or Apache-2.0. Part of <a href="https://github.com/stjude-rust-labs">St. Jude Rust Labs</a>.',
      copyright: "Copyright © 2024–present St. Jude Children's Research Hospital",
    },
  },
});
