import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Crankshaft",
  description: "Documentation related to the Crankshaft project.",
  base: "/crankshaft/",
  themeConfig: {
    nav: [{ text: "Home", link: "/" }],
    sidebar: [
      {
        text: "Overview",
        link: "/",
      },
      {
        text: "Configuration",
        link: "/configuration",
      },
      {
        text: "Backends",
        items: [{ text: "Introduction", link: "/backends/introduction" }],
      },
    ],

    socialLinks: [
      {
        icon: "github",
        link: "https://github.com/stjude-rust-labs/crankshaft",
      },
    ],
  },
});
