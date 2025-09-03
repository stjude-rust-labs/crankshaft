# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic
Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.5.0 - 09-03-2025

### Added

* Added support for canceling individual tasks from events ([#53](https://github.com/stjude-rust-labs/crankshaft/pull/53))
* Docker backend: added the caller's effective gid to containers so a
  container's user can access mounts and working directory ([#54](https://github.com/stjude-rust-labs/crankshaft/pull/54)).
* Added `monitoring` compile-time feature for enabling support for monitoring
  in `Engine` ([#49](https://github.com/stjude-rust-labs/crankshaft/pull/49)).
* Implemented starting a monitor server via the `Engine`; backends now send
  events through the broadcast channel ([#44](https://github.com/stjude-rust-labs/crankshaft/pull/44)).

### Changed

* Based events on `crankshaft-events` ([#49](https://github.com/stjude-rust-labs/crankshaft/pull/49)).
* Adds configuration for TES client retries ([#42](https://github.com/stjude-rust-labs/crankshaft/pull/42)).

## 0.4.0 - 06-04-2025

### Added

* Added support for bearer token authentication in the TES backend ([#38](https://github.com/stjude-rust-labs/crankshaft/pull/38)).

### Fixed

* Add missing error context for TES api calls ([#39](https://github.com/stjude-rust-labs/crankshaft/pull/39)).
* Fixed improper unit conversion in the TES backend ([#37](https://github.com/stjude-rust-labs/crankshaft/pull/37)).

## 0.3.0 - 05-28-2025

### Changed

* Improved backend error reporting to allow the caller to know if a task was
  canceled or preempted so that the appropriate action can be taken ([#36](https://github.com/stjude-rust-labs/crankshaft/pull/36)).

## 0.2.0 - 04-31-2025

### Changed

* Refactored how `stdout` and `stderr` are handled by backends
  ([#31](https://github.com/stjude-rust-labs/crankshaft/pull/31)).

## 0.1.0 - 04-01-2025

### Added

* Added a notification channel for the first time a task starts executing
  ([#16](https://github.com/stjude-rust-labs/crankshaft/pull/16)).
* Added support for bind mounting inputs to the Docker backend
  ([#12](https://github.com/stjude-rust-labs/crankshaft/pull/12)).
* Added cancellation support to the engine and ctrl-c handling in the examples
  ([#11](https://github.com/stjude-rust-labs/crankshaft/pull/11)).
* Added support for Docker Swarm in the docker backend
  ([#11](https://github.com/stjude-rust-labs/crankshaft/pull/11)).
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