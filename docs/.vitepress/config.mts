import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Crankshaft",
  description: "Headless Task Execution Framework Documentation.",
  base: "/crankshaft/",
  head: [
  ],
  themeConfig: {
    logo: '/header.png',
    nav: [
      { text: "Home", link: "/" },
      { text: "Guide", link: "/guide/introduction" },
      { text: "Configuration", link: "/configuration" },
      { text: "API", link: "/api/engine" },
      { text: "Examples", link: "/examples" },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Introduction', link: '/guide/introduction' },
            { text: 'Core Concepts', link: '/guide/concepts' },
            { text: 'Getting Started', link: '/guide/getting-started' },
          ]
        }
      ],
      '/configuration/': [
        {
          text: 'Configuration',
          items: [
            { text: 'Overview', link: '/configuration' },
            { text: 'Backends', link: '/configuration/backends' },
            { text: 'Docker Backend', link: '/configuration/backends/docker' },
            { text: 'TES Backend', link: '/configuration/backends/tes' },
            { text: 'Generic Backend', link: '/configuration/backends/generic' },
          ]
        }
      ],
      '/api/': [
        {
          text: 'Engine API',
          items: [
            { text: 'Engine', link: '/api/engine' },
            { text: 'Task', link: '/api/task' },
            { text: 'Execution', link: '/api/execution' },
            { text: 'Input/Output', link: '/api/io' },
            { text: 'Resources', link: '/api/resources' },
          ]
        }
      ],
      '/examples/': [
         {
           text: 'Examples',
           items: [
             { text: 'Overview', link: '/examples' },
             // Add links to specific example pages if created
           ]
         }
      ],
      '/': [
        {
          text: 'Guide',
          items: [
            { text: 'Introduction', link: '/guide/introduction' },
            { text: 'Core Concepts', link: '/guide/concepts' },
            { text: 'Getting Started', link: '/guide/getting-started' },
          ]
        },
        {
          text: 'Configuration',
          items: [
            { text: 'Overview', link: '/configuration' },
            { text: 'Backends', link: '/configuration/backends' },
          ]
        },
        {
          text: 'Engine API',
          items: [
            { text: 'Engine', link: '/api/engine' },
            { text: 'Task', link: '/api/task' },
          ]
        },
        {
          text: 'Examples',
          items: [
            { text: 'Overview', link: '/examples' },
          ]
        }
      ]
    },

    socialLinks: [
      {
        icon: "github",
        link: "https://github.com/stjude-rust-labs/crankshaft",
      },
    ],

    editLink: {
      pattern: 'https://github.com/stjude-rust-labs/crankshaft/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    },

    footer: {
        message: 'Released under the MIT OR Apache-2.0 License.',
        copyright: 'Copyright © 2024-Present St. Jude Children\'s Research Hospital'
    },
  },
  markdown: {
    lineNumbers: true
  },
  vite: {
    server: {
      fs: {
        allow: ['..']
      }
    }
  }
});
