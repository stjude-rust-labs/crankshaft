---
outline: deep
---

# Configuration 

Crankshaft can be configured using either **application-specific** or **global** configuration.

* _Application-specific_ configuration is surfaced within a downstream application's
  configuration file (conventionally, under a top-level `crankshaft` key). This
  mechanism of configuring Crankshaft is common and is generally all you need to get
  started.
* _Global_ configuration occurs in dedicated Crankshaft configuration files at various
  locations throughout the filesystem. This type of configuration is usually reserved
  for power users and allows a single configuration to be shared across all applications
  that use Crankshaft or even across multiple users.

Application-specific configuration _always_ overrides global configuration within
Crankshaft.

### File format

Whether it exists as a standalone file or a nested key within a downstream application's
configuration, the format of a Crankshaft configuration object is simple: it contains an
array of known backend configurations. Each backend configuration has a `kind` key that
determines what kind of backend the configuration contains. Each backend kind has its
own set of specific options that may be set. For example, here is a (fictional)
configuration file with three backends—two `Generic` backends and one `Docker` backend.

```toml
# A simple docker backend using the `Docker` configuration kind.
[[backends]]
name = "docker"
kind = "Docker"
max-tasks = 10

# A simple task execution service backend using the `TES` configuration kind.
[[backends]]
name = "tes"
kind = "TES"
url = "https://example.com/tes/api/v1/"
max-tasks = 10

# A fairly complex LSF HPC backend using the `Generic` configuration kind.
[[backends]]
name = "lsf"
kind = "Generic"
job-id-regex = "Job <(\\d+)>.*"
max-tasks = 10
monitor = "~/check-job-alive ~{job_id}"
monitor_frequency = 5
kill = "bkill ~{job_id}"
shell = "bash"
submit = """
    bsub
        -q ~{queue}
        -n ~{cpu}
        -cwd ~{cwd}
        -o ~{cwd}/stdout
        -e ~{cwd}/stderr
        -R "rusage[mem=~{ram_mb}] span[hosts=~{hosts}]"
        ~{command}"""

[backends.attributes]
hosts = "1"
queue = "my-queue"

[backends.defaults]
ram = 3.0

[backends.locale]
kind = "SSH"
host = "<MY_HPC_HOSTNAME>"
```

As you can see, some configuration option across the backends are the same, while others
are different. You can learn more about the kinds of backends and the various options
supported in the [Backend guide](./backends/introduction.md).

Configuration files are conventionally specified as `toml` files, but technically any
format [supported by the `config` crate](https://docs.rs/config/latest/config/) will
work.

> [!TIP]
> If you arrived at this page from a different application's documentation, they
> probably linked you here to learn what configuration options can be specified under
> the `crankshaft` key of their configuration file. If that's the case, feel free to
> move on the [Backend guide](./backends/introduction.md) to learn how to configure each
> entry within this array.

### Global configuration loading

Crankshaft uses a tiered set of global configuration sources (either files or
environment variables) to construct the final configuration used at  
runtime. The following configuration sources are loaded (if they exist) in the order
listed here. Earlier configurations values are overwritten by later configuration
values.

* A file at the canonical configuration directory on each platform.
  * **Windows.** `C:\Users\<USER>\AppData\Roaming\crankshaft\Crankshaft.toml`
  * **Mac.** `/Users/<USER>/Library/Application Support/crankshaft/Crankshaft.toml`
  * **Linux.** `/home/<USER>/.config/crankshaft/Crankshaft.toml`
* A file in the current working directory named `Crankshaft.toml`.
* Environment variables starting with `CRANKSHAFT_<VAR_NAME>`.
