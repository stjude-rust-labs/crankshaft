
# Release

Run this process for each subcrate.

  * [ ] Update version in `Cargo.toml`.
  * [ ] Run tests: `cargo test --all-features`.
  * [ ] Run linting: `cargo clippy --all-features -- -D warnings`.
  * [ ] Run fmt: `cargo +nightly fmt --check`.
  * [ ] Run doc: `cargo doc`.
  * [ ] Update each of the `CHANGELOG.md`s with version and publication date.
  * [ ] Stage changes: `git add Cargo.toml CHANGELOG.md`.
  * [ ] Create git commit:
    ```
    git commit -m "release: bumps `crankshaft-config` version to v0.1.0"
    ```
  * [ ] Create git tag:
    ```
    git tag crankshaft-config-v0.1.0
    ```
  * [ ] Publish the component crate: `cargo publish --all-features`.

After all subcrates have been released, do the following.

  * [ ] Push release: `git push && git push --tags`.
  * [ ] Make sure the CI is green for those commits.
  * [ ] Go to the Releases page in Github, create a Release for this tag, and
    copy the notes from the `CHANGELOG.md` file.