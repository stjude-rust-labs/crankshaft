<img style="margin: 0px" alt="Repository Header Image" src="./assets/repo-header.png" />
<hr/>

<p align="center">
  <p align="center">
    <a href="https://github.com/stjude-rust-labs/crankshaft/actions/workflows/CI.yml" target="_blank">
      <img alt="CI: Status" src="https://github.com/stjude-rust-labs/crankshaft/actions/workflows/CI.yml/badge.svg" />
    </a>
    <a href="https://crates.io/crates/crankshaft" target="_blank">
      <img alt="crates.io version" src="https://img.shields.io/crates/v/crankshaft">
    </a>
    <img alt="crates.io downloads" src="https://img.shields.io/crates/d/crankshaft">
    <a href="https://github.com/stjude-rust-labs/crankshaft/blob/main/LICENSE-APACHE" target="_blank">
      <img alt="License: Apache 2.0" src="https://img.shields.io/badge/license-Apache 2.0-blue.svg" />
    </a>
    <a href="https://github.com/stjude-rust-labs/crankshaft/blob/main/LICENSE-MIT" target="_blank">
      <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg" />
    </a>
  </p>

  <p align="center">
    A headless workflow execution engine for bioinformatics that supports local, cloud, and HPC.
    <br />
    <br />
    <a href="https://github.com/stjude-rust-labs/crankshaft/issues/new?assignees=&title=Descriptive%20Title&labels=enhancement">Request Feature</a>
    ·
    <a href="https://github.com/stjude-rust-labs/crankshaft/issues/new?assignees=&title=Descriptive%20Title&labels=bug">Report Bug</a>
    ·
    ⭐ Consider starring the repo! ⭐
    <br />
  </p>
</p>

## Overview

`crankshaft` is a headless workflow execution engine written in Rust: it's being
developed in coordination with the [`sprocket`] project with the goal of
enabling large-scale bioinformatics analyses. There is no associated
`crankshaft` command line tool—the end user is really engine _developers_ who
want to include it as a core task execution library in their own command line
tools.

## Guiding Principles

- `crankshaft` aims to be a **high-performance** workflow execution engine
  capable of concurrently managing and executing upwards of 20,000 concurrent
  tasks. The core focus is enabling middle- to large-scale bioinformatics
  analyses, though it can also be used to design smaller scale execution
  engines.
- `crankshaft` is **headless**, which means that it doesn't do anything on its
  own; in fact, it _must_ be driven by some external orchestration code. This
  allows the `crankshaft` library itself to focus on performance improvements
  that can be enjoyed across the entire community.
- `crankshaft` is developed **independently of any particular workflow
  language**. Though it's part of the Sprocket project, it's not based on WDL,
  and, in theory, multiple frontends based on different workflow
  languages can exist (and we hope this is the case)!

## 📚 Getting Started

### Installation

To use `crankshaft`, you'll need to install [Rust](https://www.rust-lang.org/).
We recommend using [rustup](https://rustup.rs/) to accomplish this. Once Rust is
installed, you can create a new project and add the latest version of
`crankshaft` using the following command.

```bash
cargo add crankshaft
```

Once you've added `crankshaft` to your dependencies, you should head over to the
[`/examples`](https://github.com/stjude-rust-labs/crankshaft/tree/main/crankshaft/examples)
to see how you can use the library in your projects.

## 🖥️ Development

To bootstrap a development environment, please use the following commands.

```bash
# Clone the repository
git clone git@github.com:stjude-rust-labs/crankshaft.git
cd crankshaft

# Build the crate in release mode
cargo build --release
```

## 🚧️ Tests

Before submitting any pull requests, please make sure the code passes the
following checks (from the root directory).

```bash
# Run the project's tests.
cargo test --all-features

# Run the tests for the examples.
cargo test --examples --all-features

# Ensure the project doesn't have any linting warnings.
cargo clippy --all-features

# Ensure the project passes `cargo fmt`.
cargo fmt --check

# Ensure the docs build.
cargo doc
```

## 🤝 Contributing

Contributions, issues and feature requests are welcome! Feel free to check
[issues page](https://github.com/stjude-rust-labs/crankshaft/issues).

## 📝 License

This project is licensed as either [Apache 2.0][license-apache] or
[MIT][license-mit] at your discretion.

Copyright © 2024-Present [St. Jude Children's Research Hospital](https://github.com/stjude).

[license-apache]: https://github.com/stjude-rust-labs/crankshaft/blob/main/LICENSE-APACHE
[license-mit]: https://github.com/stjude-rust-labs/crankshaft/blob/main/LICENSE-MIT
[`sprocket`]: https://github.com/stjude-rust-labs/sprocket
