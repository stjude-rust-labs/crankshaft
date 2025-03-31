# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

* Added support for respecting CPU and memory limits ([#16](https://github.com/stjude-rust-labs/crankshaft/pull/16)).
* Added support for submitting tasks via the service API for Docker Swarm (#[11](https://github.com/stjude-rust-labs/crankshaft/pull/11)).
* Adds the initial version of the crate.

### Changed

* Use `thiserror` for custom error types
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Separate `program` from `args` in container builder
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Replaced `attached` with separate stdout and stderr attach flags
  ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Updated `bollard` to an official, upstreamed crate release
  ([#29](https://github.com/stjude-rust-labs/crankshaft/pull/29)).

### Fixed

* Fixed environment variables not being set in containers ([#18](https://github.com/stjude-rust-labs/crankshaft/pull/18))
* Fixed a non-zero exit code from a container being treated as a wait error ([#16](https://github.com/stjude-rust-labs/crankshaft/pull/16)).
