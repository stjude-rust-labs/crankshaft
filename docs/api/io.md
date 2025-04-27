# Input and Output API

Define data dependencies (`Input`) and products (`Output`) for a [Task](./task.md).

## Input (`crankshaft_engine::task::Input`)

Describes a file or directory needed by a task.

### Creating an Input (`Input::builder()`)

Inputs are often wrapped in `Arc` for potential sharing between task definitions.

```rust
use crankshaft::engine::task::{Input, Contents, Type};
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

# fn main() -> eyre::Result<()> {
// --- Input Sources (Contents) ---

// 1. From Host Path (Docker/Generic bind-mount; TES sends content)
let host_path_input = Arc::new(Input::builder()
    .path("/data/ref.fa")
    .contents(Contents::Path(PathBuf::from("/host/data/genome.fa")))
    .ty(Type::File)
    .build());

// 2. From Literal Bytes (Docker/Generic write to temp file & mount; TES sends content)
let literal_input = Arc::new(Input::builder()
    .path("/app/config.ini")
    .contents(Contents::Literal("[settings]\nthreads=4".as_bytes().to_vec()))
    .ty(Type::File)
    .build());

// 3. From URL (TES uses URL directly; Docker/Generic fetch file:// only)
let url_input = Arc::new(Input::builder()
    .path("/data/reads.fq")
    .contents(Contents::url_from_str("s3://bucket/reads.fq")?) // Helper function
    .ty(Type::File)
    .build());

// --- Other Builder Fields ---
let input_with_meta = Arc::new(Input::builder()
    .name("reference_genome")
    .description("GRCh38 reference")
    .path("/ref/hg38.fa")
    .contents(Contents::Path(PathBuf::from("/data/hg38.fa")))
    .ty(Type::File)
    .read_only(true)
    .build());
# Ok(())
# }
# Input Builder Methods

- `.name(impl Into<Option<String>>):` Optional name.
- `.description(impl Into<Option<String>>):` Optional description.
- `.path(impl Into<String>):` **Required.** Destination path inside the execution environment.
- `.contents(impl Into<Contents>):` **Required.** Source of the data. See [Contents Enum](#contents-enum).
- `.ty(impl Into<Type>):` **Required.** `Type::File` or `Type::Directory`.
- `.read_only(bool):` Optional (default `true`). Hint to mount read-only.
- `.build() -> Input:` Creates the `Input`.

# Contents Enum

Defines the input source:

- `Contents::Path(PathBuf):` Path on the host running Crankshaft.  
  - **Docker/Generic:** Bind-mounted.
  - **TES:** Content read and sent inline (requires UTF-8, bad for large files).

- `Contents::Literal(Vec<u8>):` Direct byte content.  
  - **Docker/Generic:** Written to host temp file, then bind-mounted.
  - **TES:** Sent as inline content (requires UTF-8).

- `Contents::Url(Url):` URL source.  
  - **Docker/Generic:** Only `file://` URLs are fetched by Crankshaft currently. Others ignored.
  - **TES:** URL passed directly to TES service for fetching (standard for large remote files).

- `Contents::url_from_str(impl AsRef<str>) -> Result<Contents>:` Helper to create `Contents::Url`.

# Type Enum (for Input/Output)

- `Type::File:` A single file.
- `Type::Directory:` A directory. Directory support varies by backend and Contents type.

# Output (crankshaft_engine::task::Output)

Describes an expected output file or directory.

## Creating an Output (`Output::builder()`)

```rust
use crankshaft::engine::task::{Output, Type};
use url::Url;

// fn main() -> eyre::Result<()> {
    // Output file to be uploaded by TES
    let file_output = Output::builder()
        .name("alignment_bam")
        .description("Final aligned BAM")
        .path("/workdir/result.bam") // Path *inside* container where created
        .url(Url::parse("s3://results-bucket/sampleX.bam")?) // Destination URL (used by TES)
        .ty(Type::File)
        .build();

    // Output directory to be uploaded by TES
    let dir_output = Output::builder()
        .path("/workdir/qc_reports/")
        .url(Url::parse("s3://results-bucket/sampleX/qc/")?)
        .ty(Type::Directory)
        .build();
// Ok(())
// }

# Data Flow Summary

| Scenario                         | Docker/Generic Backend Handling                                           | TES Backend Handling                                    |
|-----------------------------------|---------------------------------------------------------------------------|---------------------------------------------------------|
| **Input: Contents::Path**         | Bind-mount host path to container path.                                    | Read host path, send content inline (requires UTF-8).    |
| **Input: Contents::Literal**      | Write to temp file, bind-mount temp file.                                  | Send content inline (requires UTF-8).                   |
| **Input: Contents::Url (file://)**| Fetch file, write to temp file, bind-mount.                                | Pass URL to TES service.                               |
| **Input: Contents::Url (other)**  | Ignored/Not Supported by Crankshaft fetcher.                               | Pass URL to TES service for fetching.                   |
| **Output: (File at path)**        | File appears on host if path is within a mount. URL ignored.               | TES service uploads from path to URL.                   |
