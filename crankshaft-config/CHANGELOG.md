# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic
Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.1.0 - 04-01-2025

### Added

* Added support for specifying CPU and memory limits to configuration defaults
  ([#16](https://github.com/stjude-rust-labs/crankshaft/pull/16)).
* Adds the initial version of the crate.

### Changed

* Use `thiserror` for custom error types
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Removes `#[builder(into)]` for numerical types
  ([#10](https://github.com/stjude-rust-labs/crankshaft/pull/10)).
