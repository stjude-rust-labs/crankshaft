# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.2.0 - 02-11-2026

### Changed

* Made `Monitor::start()` and `MonitorService::new()` async ([#60](https://github.com/stjude-rust-labs/crankshaft/pull/60)).

## 0.1.0 - 09-03-2025

#### Added

* Added support for canceling tasks ([#53](https://github.com/stjude-rust-labs/crankshaft/pull/53))
* Added initial server sync ([#46](https://github.com/stjude-rust-labs/crankshaft/pull/46)).
* Added utility `send_event!` macro ([#44](https://github.com/stjude-rust-labs/crankshaft/pull/44)).
* Added initial implementation ([#43](https://github.com/stjude-rust-labs/crankshaft/pull/43)).

#### Changed

* Changed protobuf definition to be based off of `crankshaft-events` ([#49](https://github.com/stjude-rust-labs/crankshaft/pull/49)).
* Removed unnecessary `metadata` field from `Event` ([#44](https://github.com/stjude-rust-labs/crankshaft/pull/44)).

#### Fixed

* Only create a `Receiver` for client subscriptions ([#51](https://github.com/stjude-rust-labs/crankshaft/pull/51)).
