---
layout: page
sidebar: false
aside: false
title: Crankshaft
titleTemplate: Task execution for workflow engines
description: Crankshaft is a headless Rust library that submits, monitors, and cancels tasks on Docker, GA4GH TES, or any scheduler that can be run from a shell. It was developed at St. Jude for the bioinformatics execution engine, Sprocket, and can be used for any kind of work.
pageClass: home-page
---

<HomeHero />

<BackendTable />

<section class="home-block steps" aria-labelledby="steps-title">
<div class="home-block__inner steps__inner">
<div class="steps__intro">
<h2 id="steps-title" class="home-block__title">Four steps from nothing to an exit code</h2>
<p class="home-block__intro">Crankshaft runs inside your Tokio runtime, and you decide when tasks are canceled. It handles scheduling, calls to the backends, and tracking each task.</p>
<ol class="steps__list">
<li><span><b>Register a backend</b>Give it a name, a kind, and a <code>max_tasks</code> limit.</span></li>
<li><span><b>Describe a Task</b>One or more executions, each with an image, a program, and arguments.</span></li>
<li><span><b>Spawn it by name</b>With a <code>CancellationToken</code> you control.</span></li>
<li><span><b>Wait for results</b>One exit status per execution, or a typed error.</span></li>
</ol>
<p class="steps__deps">Needs <code>tokio</code>, <code>tokio-util</code>, <code>nonempty</code>, and <code>anyhow</code>. <a href="./guide/getting-started">Getting started</a> has the full setup.</p>
</div>
<div class="steps__code vp-doc">

```rust
use crankshaft::Engine;
use crankshaft::config::backend::{Config, Kind, docker};
use crankshaft::engine::Task;
use crankshaft::engine::task::Execution;
use nonempty::NonEmpty;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Register a backend under a name.
    let backend = Config::builder()
        .name("docker")
        .kind(Kind::Docker(docker::Config::default()))
        .max_tasks(50)
        .build();
    let engine = Engine::default().with(backend).await?;

    // 2. Describe the work.
    let task = Task::builder()
        .name("hello")
        .executions(NonEmpty::new(
            Execution::builder()
                .images(["alpine"])?
                .program("echo")
                .args([String::from("hello, world!")])
                .build(),
        ))
        .build();

    // 3. Spawn it on the backend by name.
    let handle = engine
        .spawn("docker", task, CancellationToken::new())
        .await?;

    // 4. Wait for one result per execution.
    let results = handle.wait().await?;
    println!("{}", results.first().status);

    engine.shutdown().await;
    Ok(())
}
```

</div>
</div>
</section>

<EventStream />

<SprocketNote />
