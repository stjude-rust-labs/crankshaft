export type TesServer = {
  name: string;
  repo: string;
  runsOn: string;
  note?: string;
  // Star count at build time, or null if GitHub couldn't be reached.
  stars: number | null;
};

// Alphabetical, ignoring case.
const servers: Omit<TesServer, "stars">[] = [
  {
    name: "Funnel",
    repo: "calypr/funnel",
    runsOn: "Local, Kubernetes, HPC schedulers (Slurm, PBS, Grid Engine, HTCondor), AWS Batch, and Google Cloud Batch",
  },
  {
    name: "Planetary",
    repo: "stjude-rust-labs/planetary",
    runsOn: "Kubernetes",
    note: "Developed by the St. Jude Rust Labs team",
  },
  { name: "Poiesis", repo: "JaeAeich/poiesis", runsOn: "Kubernetes" },
  { name: "poiesisd", repo: "JaeAeich/poiesisd", runsOn: "Docker" },
  { name: "proTES", repo: "elixir-cloud-aai/proTES", runsOn: "In front of other TES servers, as a gateway" },
  { name: "TESK", repo: "elixir-cloud-aai/TESK", runsOn: "Kubernetes" },
];

declare const data: TesServer[];
export { data };

export default {
  async load(): Promise<TesServer[]> {
    const headers = { Accept: "application/vnd.github+json" };

    return Promise.all(
      servers.map(async (server) => {
        try {
          const res = await fetch(`https://api.github.com/repos/${server.repo}`, {
            headers,
            signal: AbortSignal.timeout(5000),
          });
          if (!res.ok) return { ...server, stars: null };
          const json = (await res.json()) as { stargazers_count?: number };
          return { ...server, stars: json.stargazers_count ?? null };
        } catch {
          return { ...server, stars: null };
        }
      }),
    );
  },
};
