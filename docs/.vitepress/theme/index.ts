import type { Theme } from "vitepress";
import DefaultTheme from "vitepress/theme";
import "@fontsource/barlow/latin-400.css";
import "@fontsource/barlow/latin-400-italic.css";
import "@fontsource/barlow/latin-500.css";
import "@fontsource/barlow/latin-600.css";
import "@fontsource/barlow-semi-condensed/latin-500.css";
import "@fontsource/barlow-semi-condensed/latin-600.css";
import "@fontsource-variable/jetbrains-mono";
import "./style.css";
import HomeHero from "./components/HomeHero.vue";
import BackendTable from "./components/BackendTable.vue";
import EventStream from "./components/EventStream.vue";
import SprocketNote from "./components/SprocketNote.vue";
import CopyCommand from "./components/CopyCommand.vue";
import TesServers from "./components/TesServers.vue";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("HomeHero", HomeHero);
    app.component("BackendTable", BackendTable);
    app.component("EventStream", EventStream);
    app.component("SprocketNote", SprocketNote);
    app.component("CopyCommand", CopyCommand);
    app.component("TesServers", TesServers);
  },
} satisfies Theme;
