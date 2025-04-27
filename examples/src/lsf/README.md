# LSF Generic Backend Example

This example demonstrates submitting tasks to an LSF (Load Sharing Facility) cluster using Crankshaft's `Generic` backend configured for SSH execution. This showcases how to adapt Crankshaft to common HPC schedulers.

::: danger Requires Adaptation!
This example contains hardcoded configuration specific to a hypothetical LSF setup. You **must** modify the configuration strings (`submit`, `monitor`, `kill`, `job_id_regex`, `host`) within `examples/src/lsf/main.rs` to match **your specific LSF environment**.
:::

## Prerequisites

*   **Rust & Cargo:** Ensure Rust and Cargo are installed.
*   **LSF Cluster Access:** An account on an LSF cluster accessible via SSH.
*   **SSH Agent Authentication:**
    *   An SSH agent must be running (`ssh-agent`) on the machine where you execute this example.
    *   Your SSH private key (corresponding to the public key authorized on the LSF cluster) must be added to the agent (`ssh-add`).
    *   Passwordless SSH access to the LSF login node (specified as `host` in the config) must work using the key in your agent.
*   **Monitoring Script (`check-job-alive`):**
    *   An **executable script** named `check-job-alive` must exist in your **home directory on the LSF login node** (`~/check-job-alive`).
    *   **Purpose:** Determine if an LSF job is active (running/pending).
    *   **Input:** Accepts a single argument: the LSF Job ID.
    *   **Output:**
        *   **Exit Code 0:** Job is considered *active* (e.g., `RUN`, `PEND`).
        *   **Non-Zero Exit Code:** Job is considered *finished* (e.g., `DONE`, `EXIT`) or not found.
    *   **Example Implementation (Conceptual):**
        ```bash
        #!/bin/bash
        # ~/check-job-alive
        JOB_ID=$1
        # Check if bjobs output for the specific job contains RUN or PEND status
        bjobs -noheader -o stat $JOB_ID | grep -qE 'RUN|PEND'
        # grep exits 0 if found (active), 1 if not found (finished/failed)
        exit $?
        ```
        *(Adjust the `bjobs` command and status checks based on your LSF version and needs)*. Make sure this script is executable (`chmod +x ~/check-job-alive`).

## Configuration (Hardcoded in Example)

The example defines its backend configuration as a hardcoded YAML string within `src/lsf/main.rs`. **You must modify this string.**

**Key parts to adapt:**

*   `locale.host`: Change `"hpc"` to your LSF login node's hostname or IP.
*   `locale.username` (Optional): Add if your HPC username differs from your local username.
*   `submit`: Modify the `bsub` command:
    *   `-q compbio`: Change to your target queue.
    *   `-o`, `-e`: Adjust output/error file paths if needed.
    *   `-n`, `-R`: Ensure resource request syntax (`~{cpu}`, `~{ram_mb}`) matches your LSF setup.
*   `monitor`: Ensure `~/check-job-alive ~{job_id}` correctly calls your monitoring script.
*   `kill`: Ensure `bkill ~{job_id}` is the correct command.
*   `job_id_regex`: **Crucial.** Verify `'Job <(\d+)>.*'` correctly extracts the numeric Job ID from *your* `bsub` command's standard output. The ID must be in the first capture group `(\d+)`.
*   `attributes` (Optional): Add site-specific placeholders if needed (e.g., project codes).

## Running the Example

1.  **Modify Configuration:** Edit `examples/src/lsf/main.rs` and adapt the hardcoded `CONFIG` string.
2.  **Setup SSH Agent:** Ensure your agent is running and the correct key is added (`ssh-add`).
3.  **Place Monitor Script:** Ensure your `~/check-job-alive` script is present and executable on the LSF login node.
4.  **Run:** Navigate to the root of the `crankshaft` repository and execute:

    ```bash
    cargo run --release --bin lsf -- --n-jobs 50
    ```

### Command Line Arguments

*   `--n-jobs <NUMBER>`: (Optional) Specifies the total number of identical tasks (LSF jobs) to submit.
    *   Default: `1000`

## What it Does

1.  **Parses Config:** Loads the (modified) hardcoded YAML string.
2.  **Initializes Engine:** Creates the `Engine` and the `Generic` backend, establishing an SSH connection.
3.  **Defines Task:** Creates a simple task (`echo "hello, world!"`).
4.  **Spawns Tasks:** For each task:
    *   Substitutes placeholders (`~{...}`) into the `submit` command.
    *   Executes `submit` via SSH.
    *   Extracts the LSF Job ID using `job_id_regex`.
    *   Periodically executes the `monitor` command via SSH (passing the Job ID).
    *   Stops monitoring when the `monitor` script exits non-zero.
    *   Executes `kill` via SSH if cancelled.
5.  **Displays Output:** Shows a progress bar. Prints the final exit status *of the monitoring process*. Stdout/stderr shown are from the *final monitor command*, not the LSF job itself.
