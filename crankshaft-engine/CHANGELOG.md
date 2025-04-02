# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic
Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.1.0 - 04-01-2025

### Added

* Added a notification channel for the first time a task starts executing
  ([#16](https://github.com/stjude-rust-labs/crankshaft/pull/16)).
* Added support for bind mounting inputs to the Docker backend
  ([#12](https://github.com/stjude-rust-labs/crankshaft/pull/12)).
* Added cancellation support to the engine and ctrl-c handling in the examples
  (#[11](https://github.com/stjude-rust-labs/crankshaft/pull/11)).
* Added support for Docker Swarm in the docker backend
  (#[11](https://github.com/stjude-rust-labs/crankshaft/pull/11)).
* Adds the initial version of the crate.
* Adds basic auth to the TES examples
  ([[#6](https://github.com/stjude-rust-labs/crankshaft/issues/6)]).

### Changed

* Use `thiserror` for custom error types
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Remove progress bar from `Engine` and made `Runner` implement `Sync`
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Fixed a hang in the name generator when the bloom filter becomes saturated;
  replace the bloom filter implementation with a growable one
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Adds `Resource::builder()` and `Output::builder()` to match the
  `Input::builder`.
* Multiple revisions to the TES backend
  ([#9](https://github.com/stjude-rust-labs/crankshaft/issues/9)).
* Better handling for URL contents in inputs.
* Swaps out most of the bespoke builders for `bon`.
* Removes `#[builder(into)]` for numerical types
  ([#10](https://github.com/stjude-rust-labs/crankshaft/pull/10)).

### Fixed

* The Docker backend now ensures task execution images are present locally
  before creating any containers
  ([#12](https://github.com/stjude-rust-labs/crankshaft/pull/12)).