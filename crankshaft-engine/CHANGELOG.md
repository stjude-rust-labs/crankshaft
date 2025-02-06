# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

* Adds the initial version of the crate.
* Adds basic auth to the TES examples
  ([[#6](https://github.com/stjude-rust-labs/crankshaft/issues/6)]).

### Changed

* Use `thiserror` for custom error types ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Remove progress bar from `Engine` and made `Runner` implement `Sync` ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
* Fixed a hang in the name generator when the bloom filter becomes saturated;
  replace the bloom filter implementation with a growable one ([#8](https://github.com/stjude-rust-labs/crankshaft/pull/8)).
