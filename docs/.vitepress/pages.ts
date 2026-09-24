// Browser-safe page map shared by the site config and theme components.

export type DocPage = {
  title: string;
  link: string;
  section: string;
};

export type Backend = {
  // Balloon number in the hero drawing.
  item: number;
  // Name used for the backend in the synthetic event stream.
  key: string;
  title: string;
  link: string;
  summary: string;
  drivenBy: string;
  images: string;
  callout: string;
};

export const sections = ["Get started", "Concepts", "Backends", "Events"] as const;

export const pages: DocPage[] = [
  { title: "Introduction", link: "/guide/introduction", section: "Get started" },
  { title: "Getting started", link: "/guide/getting-started", section: "Get started" },
  { title: "Engine", link: "/concepts/engine", section: "Concepts" },
  { title: "Tasks", link: "/concepts/tasks", section: "Concepts" },
  { title: "Configuration", link: "/concepts/configuration", section: "Concepts" },
  { title: "Docker", link: "/backends/docker", section: "Backends" },
  { title: "TES", link: "/backends/tes", section: "Backends" },
  { title: "Generic", link: "/backends/generic", section: "Backends" },
  { title: "Events", link: "/observe/events", section: "Events" },
  { title: "Monitoring", link: "/observe/monitoring", section: "Events" },
];

export const backends: Backend[] = [
  {
    item: 1,
    key: "docker",
    title: "Docker",
    link: "/backends/docker",
    summary:
      "Runs each execution as a container on the local Docker daemon, or as a Swarm service if the daemon is a Swarm manager.",
    drivenBy: "The Docker Engine API",
    images: "Pulled, with fallbacks tried in order",
    callout: "Local daemon or Swarm",
  },
  {
    item: 2,
    key: "tes",
    title: "TES",
    link: "/backends/tes",
    summary:
      "Sends each task to a GA4GH Task Execution Service, such as Funnel, TESK, or a cloud provider's. The server runs the containers.",
    drivenBy: "HTTP, with basic or bearer auth",
    images: "Passed to the TES server",
    callout: "GA4GH Task Execution Service",
  },
  {
    item: 3,
    key: "lsf",
    title: "Generic",
    link: "/backends/generic",
    summary:
      "Runs tasks through submit, monitor, and kill commands you write. That's enough for LSF, Slurm, or PBS, locally or over SSH.",
    drivenBy: "Your shell commands",
    images: "Ignored",
    callout: "LSF, Slurm, and others",
  },
];

export const links = {
  github: "https://github.com/stjude-rust-labs/crankshaft",
  docsrs: "https://docs.rs/crankshaft",
  crates: "https://crates.io/crates/crankshaft",
  sprocket: "https://sprocket.bio",
  slack: "https://join.slack.com/t/openwdl/shared_invite/zt-ctmj4mhf-cFBNxIiZYs6SY9HgM9UAVw",
  examples: "https://github.com/stjude-rust-labs/crankshaft/tree/main/examples",
};
