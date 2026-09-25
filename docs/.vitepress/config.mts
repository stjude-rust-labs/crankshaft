import { defineConfig, type DefaultTheme } from "vitepress";
import { links, pages, sections } from "./pages";
import { redlineDark, redlineLight } from "./redline-theme";

const sidebar: DefaultTheme.SidebarItem[] = sections.map((section) => ({
  text: section,
  items: pages.filter((p) => p.section === section).map((p) => ({ text: p.title, link: p.link })),
}));

const siteUrl = "https://stjude-rust-labs.github.io/crankshaft/";
const ogImage = `${siteUrl}og.png`;
const ogImageAlt =
  "Crankshaft: task execution for workflow engines. A drawing of a crankshaft with three numbered throws for the Docker, TES, and Generic backends.";

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
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: "Crankshaft" }],
    ["meta", { property: "og:locale", content: "en_US" }],
    ["meta", { property: "og:image", content: ogImage }],
    ["meta", { property: "og:image:type", content: "image/png" }],
    ["meta", { property: "og:image:width", content: "2400" }],
    ["meta", { property: "og:image:height", content: "1260" }],
    ["meta", { property: "og:image:alt", content: ogImageAlt }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:image", content: ogImage }],
    ["meta", { name: "twitter:image:alt", content: ogImageAlt }],
  ],
  sitemap: { hostname: siteUrl },
  transformHead({ pageData, title, description }) {
    if (pageData.isNotFound) return [];
    const url = siteUrl + pageData.relativePath.replace(/(^|\/)index\.md$/, "$1").replace(/\.md$/, "");
    return [
      ["link", { rel: "canonical", href: url }],
      ["meta", { property: "og:url", content: url }],
      ["meta", { property: "og:title", content: title }],
      ["meta", { property: "og:description", content: description }],
      ["meta", { name: "twitter:title", content: title }],
      ["meta", { name: "twitter:description", content: description }],
    ];
  },
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
