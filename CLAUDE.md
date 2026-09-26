# three-rs

Bare-repo worktree layout. `.bare/` is the repository; `main/` is always checked out on `main`; every other branch is a sibling directory named exactly after it: `three-rs/<branch>/`.

Never `git switch`, `git checkout` or commit in `main/`. It is the clean copy to read and diff against.

Start a branch from `three-rs/`, not from inside a worktree: `git worktree add <branch> -b <branch> main`.

When a branch is done: `git worktree remove <branch>` from `three-rs/`. The branch itself stays.

Sessions start inside a worktree (`main/` or `<branch>/`), never in `three-rs/` itself.

To recreate the layout on a fresh machine: `git clone --bare <url> .bare && echo 'gitdir: ./.bare' > .git && git worktree add main main`, then give every worktree one cargo cache: `mkdir -p .cargo && printf '[build]\ntarget-dir = "%s/.target"\n' "$PWD" > .cargo/config.toml`. Cargo reads `.cargo/config.toml` from any directory below `three-rs/`, so a new worktree starts warm instead of recompiling every dependency into its own `target/`; builds in the shared directory serialise on cargo's lock, and it only grows, so `cargo clean` it now and then.

