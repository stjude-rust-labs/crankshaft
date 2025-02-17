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
      {},
    ],

    socialLinks: [
      {
        icon: "github",
        link: "https://github.com/stjude-rust-labs/crankshaft",
      },
    ],
  },
});
