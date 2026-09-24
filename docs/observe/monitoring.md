---
title: Monitoring
description: Expose the event stream over gRPC and watch tasks live in the terminal console.
---

# Monitoring

<p class="lede">With the <code>monitoring</code> feature, the engine serves its event stream over gRPC. Point the Crankshaft console at it to watch tasks in a terminal, or write your own client.</p>

## Turn it on

The feature needs a protobuf compiler at build time. Follow the [protobuf installation guide](https://protobuf.dev/installation/) first (on macOS, `brew install protobuf`).

```sh
cargo add crankshaft --features monitoring
```

Then create the engine with an address to listen on.

```rust
use crankshaft::Engine;

let engine = Engine::new_with_monitoring("127.0.0.1:8080".parse()?).await;
```

Everything else is the same as `Engine::default()`. `engine.shutdown()` stops the server.

::: warning The monitor has no authentication
Anyone who can reach the address can see task names and cancel tasks. Bind to `127.0.0.1` unless the network in front of it is trusted.
:::

## The gRPC service

The service is `crankshaft.monitor.Monitor`, defined in `crankshaft-monitor/src/proto/monitor.proto`.

| Method | Returns | Use it to |
| --- | --- | --- |
| `SubscribeEvents` | A stream of `Event` | Follow the [event stream](/observe/events) live. Each message has a timestamp. |
| `GetServiceState` | Every task and its events so far | Catch up when a client connects mid-run. |
| `CancelTask(id)` | Nothing | Cancel a task by its id. |

## The console

`crankshaft-console` is a terminal UI in this repository. It connects to `http://localhost:8080`, shows every task and its latest state, and lets you cancel them.

```sh
# In one terminal, run an engine with the monitor on.
cargo run --release --bin docker-monitored

# In another, from the repository root, start the console.
cargo run --release -p crankshaft-console
```

| Key | Action |
| --- | --- |
| `j` or <kbd>↓</kbd> | Next task |
| `k` or <kbd>↑</kbd> | Previous task |
| `t` | Tasks view |
| `c` | Cancel the selected task, then `y` to confirm or `n` / <kbd>Esc</kbd> to back out |
| <kbd>Ctrl</kbd>+`c` or <kbd>Ctrl</kbd>+`d` | Quit |

The console always connects to port 8080 on localhost, so start the engine there.

## Reference

- [`crankshaft-monitor`](https://docs.rs/crankshaft-monitor) on docs.rs
