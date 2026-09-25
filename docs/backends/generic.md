---
title: Generic
description: Run tasks on LSF, Slurm, or any system you can drive with shell commands, locally or over SSH.
---

# Generic

<p class="lede">The Generic backend runs tasks through commands you write: one to submit a job, one to check on it, and one to kill it. That's enough to drive LSF, Slurm, PBS, or a plain shell, on this machine or over SSH.</p>

## Configure

This configuration submits to LSF over SSH.

```yaml
backends:
  - name: lsf
    kind: Generic
    max-tasks: 10
    locale:
      kind: SSH
      host: hpc.example.org
      port: 22
    shell: bash
    submit: |
      bsub
        -q compbio
        -n ~{cpu}
        -cwd ~{cwd}
        -o ~{cwd}/stdout.lsf
        -e ~{cwd}/stderr.lsf
        -R "rusage[mem=~{ram_mb}] span[hosts=~{hosts}]"
        ~{command}
    job-id-regex: Job <(\d+)>.*
    monitor: ~/check-job-alive ~{job_id}
    monitor-frequency: 5
    kill: bkill ~{job_id}
    attributes:
      hosts: '1'
    defaults:
      ram: 3
```

| Option | Default | Meaning |
| --- | --- | --- |
| `submit` | required | Command that submits one execution. |
| `monitor` | required | Command that exits `0` while the job is still alive. |
| `kill` | required | Command that stops the job when the task is canceled. |
| `job-id-regex` | none | Pattern to find the job id in `submit`'s output. Capture group 1 is the id. |
| `monitor-frequency` | `5` | Seconds between `monitor` runs. |
| `attributes` | none | Extra `~{name}` values you want to use in your commands. |
| `locale` | local | `{ kind: Local }` or `{ kind: SSH, host, port, username }`. |
| `shell` | `bash` | `bash` or `sh`. Commands run through it. |
| `max-attempts` | `4` | Over SSH, how many times to try opening a channel before giving up. |

::: warning Always set `port` for SSH
When reading a configuration file, the SSH locale needs `port` even if it's `22`. Leaving it out fails with `missing field port`. `username` is optional.
:::

## Placeholders

Commands can use `~{name}` placeholders. Crankshaft fills them for each execution; a placeholder with no value is an error.

| Placeholder | Value |
| --- | --- |
| `~{command}` | The execution's program and arguments, shell-quoted. |
| `~{cwd}` | The execution's `work_dir`. |
| `~{job_id}` | The id captured by `job-id-regex`. Only in `monitor` and `kill`. |
| `~{cpu}`, `~{cpu_limit}` | CPU cores. |
| `~{ram}`, `~{ram_limit}` | Memory in GiB. |
| `~{ram_mb}` | Memory in MiB (`ram × 1024`). |
| `~{disk}`, `~{disk_mb}` | Disk in GiB, or MiB. |
| `~{gpu}` | GPU count. |
| `~{preemptible}` | `true` or `false`. |
| Anything in `attributes` | The value you set. |

Resource placeholders come from the task's resources, falling back to the backend's `defaults` and then to `Resources::default()`. See [Configuration](/concepts/configuration#defaults).

## How a job is tracked

1. **Submit.** The backend runs `submit` and emits `TaskStarted`. This happens when the job is submitted, so the job may still be queued.
2. **Find the id.** If `job-id-regex` is set, the backend matches it against `submit`'s standard output and saves group 1 as `~{job_id}`. No match is an error.
3. **Monitor.** Every `monitor-frequency` seconds it runs `monitor`. Exit `0` means "still running". The first non-zero exit means the job is over, and that exit status becomes the execution's result.
4. **Kill on cancel.** If the task is canceled while it's being monitored, the backend runs `kill`.

Without `job-id-regex`, there's no monitoring. `submit` is treated as the job itself, and its exit status is the result. That's the simplest way to run commands directly on a machine.

::: tip Write a monitor that reports the job's real exit code
Because the monitor's exit status becomes the result, a good monitor exits `0` while the job runs, then exits with the job's own exit code. A monitor that just checks whether the job exists will report every job as failed.
:::

## Things to know

- **Images are ignored.** Generic backends run commands directly, so any images on an execution are skipped with a warning. If your cluster uses Singularity or Apptainer, call it from `submit`.
- **Inputs and outputs aren't moved.** The backend doesn't stage files. Use paths that the cluster can already see, such as a shared filesystem.
- Each execution in a task is a separate job, submitted after the previous one finishes.

## Example

`examples/src/lsf/main.rs` in the repository drives LSF with a configuration like the one above (`cargo run --release --bin lsf`). It expects a `check-job-alive` script in your home directory on the cluster.
