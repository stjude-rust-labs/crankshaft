---
layout: home

hero:
  name: "Crankshaft"
  text: "Headless Task Execution Framework"
  tagline: High-performance task execution supporting local, cloud, and HPC environments, built in Rust.
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: View on GitHub
      link: https://github.com/stjude-rust-labs/crankshaft

features:
  - title: High Performance
    icon: 🚀
    details: Designed for large-scale concurrent task execution, leveraging Rust's performance and Tokio's async capabilities.
  - title: Flexible Backends
    icon: ⚙️
    details: Supports Docker, GA4GH TES, and generic HPC/remote execution via SSH through simple configuration.
  - title: Headless Library
    icon: 🏗️
    details: Integrates as a core library into your applications, providing execution capabilities without imposing UI or workflow language specifics.
  - title: Configurable
    icon: 🛠️
    details: Define execution environments, resource defaults, and backend behavior using TOML files or environment variables.
---

## What is Crankshaft?

`crankshaft` is a **headless task execution framework** written in Rust. It provides a robust and performant library for developers building applications (like workflow managers) that need to run computational tasks across diverse environments such as:

*   **Local Machines:** Via Docker containers.
*   **Cloud Platforms:** Using the GA4GH Task Execution Service (TES) API.
*   **High-Performance Computing (HPC) Clusters:** Through a configurable `Generic` backend, often utilizing SSH.

It focuses on the core mechanics of task submission, monitoring, resource management, and concurrency, allowing the consuming application to handle the higher-level workflow logic and user interface.

## Why Crankshaft?

*   **Performance:** Built with Rust and Tokio for efficient, concurrent task handling suitable for large-scale bioinformatics or similar workloads.
*   **Flexibility:** Abstract away backend differences. Write your task logic once and run it on Docker, TES, or HPC with only configuration changes.
*   **Integration:** Designed as a library (`lib.rs`) to be embedded within larger Rust applications, giving you full control over the user experience and workflow definition.
*   **Extensibility:** While providing core backends, the architecture allows for potential future expansion.

Ready to dive in? Check out the [**Getting Started**](/guide/getting-started) guide.
