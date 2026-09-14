# Releasing

The checklist for a crates.io release. Two crates live in this workspace
and version independently: `three-rs` and `sdf-text` (depends on
`three-rs`). A release ships whichever of them changed. (`d3-hierarchy`
has its own repository and releases since #63.)

Everything here is done by hand except the last step: pushing the annotated
tag is what creates the GitHub Release, through
`.github/workflows/release.yml`. The tag message is the release notes, so
write it as one.

## 1. Decide what ships

- Which crates changed since their last tag. `git log v0.1.2.. -- sdf-text`
  answers it for the member crate.
- Which bump each one is. At 0.x a minor bump is the breaking one and a
  patch bump promises compatibility. Run `cargo semver-checks` on each
  crate that changed (#27; `cargo install cargo-semver-checks`) and take
  the bump it says is honest, not the one that was planned.
- If `three-rs` takes a minor bump, `sdf-text` must ship too, with its
  `three-rs = { path = "..", version = "..." }` dependency moved to the new
  minor. A patch bump of `three-rs` leaves `sdf-text` alone; its caret
  dependency already covers it.

## 2. Gates

On a branch, from a clean tree, on a machine with a Vulkan device:

```sh
cargo fmt --all --check
cargo test -p sdf-text --lib
cargo test -p three-rs --lib
cargo test --workspace                       # the GPU renderer tests and the e2e grader
cargo test -p sdf-text -- --test-threads=1   # the SDF text gates, on the GPU
cargo doc --workspace --no-deps
cargo publish --dry-run -p <crate>           # for each crate that ships, in the order in step 4
```

The e2e run is the whole ladder, not the rungs the release touched. If any
graded number in the README's table moved, the README is updated in the
release PR with the new number and the reason.

`cargo publish --dry-run` builds the packaged crate as crates.io will see
it. Any failure is new.

## 3. The release commit

One commit per release, on a branch, merged to `main` like any other change:

- The `version` in each shipping crate's `Cargo.toml`, and `sdf-text`'s
  dependency on `three-rs` if step 1 said so. `cargo build` refreshes
  `Cargo.lock`; commit that too.
- `CHANGELOG.md`: rename *Unreleased* to the version and date, and open a
  fresh *Unreleased* above it (#19; until the file exists, the tag message
  in step 5 is the changelog).
- The README's status line and graded table, if either changed.

Subject: `three-rs 0.1.2: <what it is, in one line>`, as the previous
releases did. The body names the issues and PRs that ship in it. For a
member crate on its own, `sdf-text 0.1.1: ...`.

## 4. Publish

From the merge commit on `main`, in dependency order, only the crates that
ship:

```sh
cargo publish -p three-rs
cargo publish -p sdf-text        # after three-rs is visible on crates.io
```

`sdf-text` resolves its `three-rs` dependency from the registry when it
packages, so the new `three-rs` has to be there first; the index usually
catches up within a minute.

A publish cannot be undone. `cargo yank` withdraws a version from new
resolutions but the number is spent; a broken release is followed by a
patch release, not re-published.

## 5. Tag, and the Release makes itself

On the merge commit on `main`, an annotated tag named `v<three-rs version>`
for a release that includes `three-rs`, or `<crate>-v<version>` for a member
crate on its own:

```sh
git tag -a v0.1.2 -m "three-rs 0.1.2: <the release commit's subject>" -m "<the notes>"
git push origin v0.1.2
```

The message is the release notes as they will appear on GitHub. First line
is the title; the body says what changed for a user, which crates and
versions shipped (with their crates.io links), and any known limitation,
in the shape of the 0.1.0 notes. Write it with `-m` twice or with
`git tag -a` and an editor; a lightweight tag fails the workflow on
purpose, because it carries no notes.

Pushing the tag runs `.github/workflows/release.yml`, which creates the
GitHub Release from the tag message. Check the Actions tab; if the run
failed, the same thing by hand is:

```sh
gh release create v0.1.2 --verify-tag --notes-from-tag
```

## 6. Afterwards

- Close the issues the release shipped, with a comment naming the version,
  if the release commit did not close them.
- Clear the `release-blocker` label from anything that carried it.
- Check the docs.rs build for each crate published (it lags a few minutes).
