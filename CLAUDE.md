# three-rs

This describes the maintainer's bare-repo worktree layout. A plain `git clone` works and needs none of it.

Bare-repo worktree layout. `.bare/` is the repository; `main/` is always checked out on `main`; every other branch is a sibling directory named exactly after it: `three-rs/<branch>/`.

Never `git switch`, `git checkout` or commit in `main/`. It is the clean copy to read and diff against.

Start a branch from `three-rs/`, not from inside a worktree: `git worktree add <branch> -b <branch> main`.

When a branch is done: `git worktree remove <branch>` from `three-rs/`. The branch itself stays.

Sessions start inside a worktree (`main/` or `<branch>/`), never in `three-rs/` itself.

Build in the worktree's own `target/`. Never set `CARGO_TARGET_DIR`, and never put a target dir under the session scratchpad. A scratchpad target outlives its worktree, is invisible to anyone sizing the project, and a debug build here runs to tens of gigabytes; separate per-worktree scratchpad targets filled the disk on 2026-10-04. The in-repo `target/` is removed with the worktree, and sccache (the global `rustc-wrapper`) keeps a fresh worktree's first build cheap.

To recreate the layout on a fresh machine: `git clone --bare <url> .bare && echo 'gitdir: ./.bare' > .git && git worktree add main main`.

