# Task API (`crankshaft_engine::Task`)

A `Task` represents a complete unit of computational work, bundling metadata, resources, inputs, outputs, and execution steps.

## Creating a Task (`Task::builder()`)

Use the fluent `TaskBuilder` API.

```rust
use crankshaft::engine::Task;
use crankshaft::engine::task::{Execution, Input, Output, Resources, Contents, Type};
use nonempty::NonEmpty;
use url::Url;
use std::sync::Arc;
use std::path::PathBuf;
use indexmap::indexmap;

# fn main() -> eyre::Result<()> {
let task = Task::builder()
    // --- Metadata (Optional) ---
    .name("bwa_alignment_sampleX")
    .description("Aligns sampleX reads to reference")

    // --- Inputs (Optional) ---
    .inputs([
        Arc::new(Input::builder()
            .path("/inputs/ref.fa")
            .contents(Contents::Path(PathBuf::from("/host/data/hg38.fa")))
            .ty(Type::File)
            .build()),
        Arc::new(Input::builder()
            .path("/inputs/reads.fq")
            .contents(Contents::url_from_str("s3://bucket/reads.fq")?)
            .ty(Type::File)
            .build()),
    ])

    // --- Outputs (Optional) ---
    .outputs([
        Output::builder()
            .path("/outputs/aligned.bam")
            .url(Url::parse("s3://bucket/results/aligned.bam")?)
            .ty(Type::File)
            .build(),
    ])

    // --- Resources (Optional) ---
    .resources(Resources::builder().cpu(8.0).ram(32.0).build())

    // --- Shared Volumes (Optional, Docker specific) ---
    .volumes(["/scratch".to_string()])

    // --- Executions (Required) ---
    .executions(
        NonEmpty::new(
            Execution::builder()
                .image("biocontainers/bwa:v0.7.17_cv1")
                .program("bwa")
                .args(["mem", "-t", "8", "/inputs/ref.fa", "/inputs/reads.fq"])
                .work_dir("/outputs")
                .stdout("aligned.sam")
                .env(indexmap!{"BWA_OPT".into() => "-M".into()})
                .build()
        )
    )
    .build(); // Finalize
# Ok(())
# }

# Task Builder Methods

## Builder Methods

- `.name(impl Into<Option<String>>)`:
  Sets optional task name.
- `.description(impl Into<Option<String>>)`:
  Sets optional description.
- `.inputs(impl IntoIterator<Item = Arc<Input>>)`:
  Sets task Inputs. Use `Arc<Input>` for potential sharing.
- `.outputs(impl IntoIterator<Item = Output>)`:
  Sets task Outputs.
- `.resources(impl Into<Option<Resources>>)`:
  Sets task Resources. Overrides backend defaults.
- `.executions(impl Into<NonEmpty<Execution>>)`:
  **Required.** Sets one or more Execution steps.
- `.volumes(impl IntoIterator<Item = String>)`:
  Sets shared volume paths inside the container (primarily for Docker).
- `.build() -> Task`:
  Creates the `Task` object.

---

## Accessing Task Properties

Use getter methods on a `Task` instance:

- `.name() -> Option<&str>`
- `.description() -> Option<&str>`
- `.inputs() -> impl Iterator<Item = Arc<Input>>`
- `.outputs() -> impl Iterator<Item = &Output>`
- `.resources() -> Option<&Resources>`
- `.executions() -> impl Iterator<Item = &Execution>` (iterates over the NonEmpty list)
- `.shared_volumes() -> impl Iterator<Item = &str>`

---

## Task Lifecycle

- **Definition**: Create via `Task::builder()`.
- **Submission**: Pass to `engine.spawn()`.
- **Execution**: Engine routes to backend; backend interprets and runs Execution steps.
- **Completion**: `TaskHandle::wait()` resolves with `Result<NonEmpty<Output>>`.
