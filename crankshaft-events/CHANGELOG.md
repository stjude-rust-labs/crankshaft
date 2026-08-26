# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

#### Added

* Added the `TaskResourceUsage` event, carrying a cumulative
  `TaskResourceUsage` snapshot of a task's observed resource utilization
  (maximum/average resident memory, total/user/system CPU time, and disk
  used); backends that can observe utilization emit it zero or more times
  over a task's lifetime, and the last snapshot received is authoritative
  ([#86](https://github.com/stjude-rust-labs/crankshaft/pull/86)).
* Added an optional `serde` feature that derives `Serialize`/`Deserialize`
  for `TaskResourceUsage`
  ([#86](https://github.com/stjude-rust-labs/crankshaft/pull/86)).
* Added `ImagePullStarted`, `ImagePullFinished`, and `ImagePullFailed` events ([#82](https://github.com/stjude-rust-labs/crankshaft/pull/82)).

## 0.1.0 - 09-03-2025

#### Added

* Added cancellation token to task created event ([#53](https://github.com/stjude-rust-labs/crankshaft/pull/53))
* Added initial definition of Crankshaft events ([#49](https://github.com/stjude-rust-labs/crankshaft/pull/49)).
