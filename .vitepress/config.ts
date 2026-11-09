import { defineConfig } from "vitepress";
import llms from "vitepress-plugin-llms";

export default defineConfig({
  title: "lasius",
  description:
    "Workspace gardener for GitHub orgs — clone, sync, inspect, and generate editor workspace files for every repo of an org under one root.",
  base: "/lasius/",
  head: [
    ["link", { rel: "icon", href: "/lasius/favicon.svg" }],
  ],
  themeConfig: {
    nav: [
      { text: "Guide", link: "/getting-started" },
      { text: "Reference", link: "/reference/commands" },
      { text: "GitHub", link: "https://github.com/9aia/lasius" },
    ],
    sidebar: [
      {
        text: "Start",
        items: [
          { text: "Introduction", link: "/" },
          { text: "Getting started", link: "/getting-started" },
        ],
      },
      {
        text: "Tutorials",
        items: [
          { text: "Bootstrap a workspace repo", link: "/tutorials/bootstrap" },
        ],
      },
      {
        text: "How-to",
        items: [
          { text: "Include private repos", link: "/how-to/private-repos" },
          { text: "Exclude repos", link: "/how-to/exclude-repos" },
          { text: "Run commands everywhere", link: "/how-to/exec-everywhere" },
          { text: "Embed via serve", link: "/how-to/serve" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "Commands", link: "/reference/commands" },
          { text: "lasius.toml", link: "/reference/config" },
          { text: "serve wire protocol", link: "/reference/serve" },
        ],
      },
      {
        text: "Explanation",
        items: [{ text: "Architecture", link: "/explanation/architecture" }],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/9aia/lasius" }],
  },
  plugins: [llms()],
});
