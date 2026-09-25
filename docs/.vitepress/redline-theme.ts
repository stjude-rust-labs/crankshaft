// Code themes for the Redline docs: ink on paper, with redline keywords.
type Palette = { bg: string; fg: string; comment: string; keyword: string; string: string; number: string; type: string; fn: string; punct: string; attr: string };

function theme(name: string, type: "light" | "dark", p: Palette) {
  return {
    name,
    type,
    colors: { "editor.background": p.bg, "editor.foreground": p.fg },
    tokenColors: [
      { settings: { foreground: p.fg, background: p.bg } },
      { scope: ["comment", "punctuation.definition.comment"], settings: { foreground: p.comment, fontStyle: "italic" } },
      {
        scope: ["keyword", "storage", "storage.type", "storage.modifier", "keyword.control", "keyword.other"],
        settings: { foreground: p.keyword },
      },
      { scope: ["string", "string.quoted", "markup.inline.raw"], settings: { foreground: p.string } },
      { scope: ["constant.numeric", "constant.language", "constant.character"], settings: { foreground: p.number } },
      {
        scope: ["entity.name.type", "support.type", "entity.name.namespace", "entity.name.type.rust", "support.class"],
        settings: { foreground: p.type },
      },
      { scope: ["entity.name.function", "support.function", "meta.function-call"], settings: { foreground: p.fn, fontStyle: "bold" } },
      { scope: ["variable", "variable.other"], settings: { foreground: p.fg } },
      { scope: ["entity.name.tag", "support.type.property-name", "entity.other.attribute-name"], settings: { foreground: p.type } },
      { scope: ["punctuation", "keyword.operator"], settings: { foreground: p.punct } },
      { scope: ["meta.attribute", "entity.name.function.macro"], settings: { foreground: p.attr } },
    ],
  };
}

export const redlineLight = theme("redline-light", "light", {
  bg: "#ffffff",
  fg: "#1c1f24",
  comment: "#6a707a",
  keyword: "#a31438",
  string: "#2d6a44",
  number: "#8a4b00",
  type: "#245a93",
  fn: "#1c1f24",
  punct: "#5a606a",
  attr: "#6a707a",
});

export const redlineDark = theme("redline-dark", "dark", {
  bg: "#1a1d22",
  fg: "#e7e9ec",
  comment: "#8a909a",
  keyword: "#ff86a1",
  string: "#9fd6ae",
  number: "#f0b47c",
  type: "#92bdf0",
  fn: "#ffffff",
  punct: "#b3b8c0",
  attr: "#8a909a",
});
