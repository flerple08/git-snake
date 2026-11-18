# Getting Started

Three ways in: a snake on your profile README, the command line, and the library.
Each stands alone.

## On Your Profile README

GitHub shows the README of the repository named after your username, such as
`octocat/octocat`, on your profile page. To give it a snake that redraws every
night:

1. Copy [examples/profile-workflow.yml](../examples/profile-workflow.yml) into
   that repository as `.github/workflows/snake.yml`.
2. Embed the snake in its `README.md`:

   ```html
   <img src="snake/snake.gif" width="256" alt="My snake" />
   ```

   The GIF is drawn at exactly twice the snake's pixel size, so `width="256"`
   shows it at native size without blurring.

3. Run the workflow once from the repository's Actions tab, so the first snake
   appears without waiting for midnight.

The workflow runs just after midnight UTC, so each snake shows the day that just
ended, and again whenever the workflow file changes. Its `git-snake` step is this
repository's Action. It collects your public activity with the workflow's
built-in token and renders `snake.gif`, `snake.png`, and `preview.html` into `snake/`.
The Action never commits. The workflow's last step commits `snake/snake.gif` only
when it changed, so an unchanged snake adds no commit.

| Input        | Default              | Meaning                                                   |
| ------------ | -------------------- | --------------------------------------------------------- |
| `login`      | The repository owner | Account whose snake to draw                                 |
| `morph`      | `normal`             | `normal`, `pastel`, `banana`, `albino`, `axanthic`, or `cinnamon` |
| `output-dir` | `snake`                | Folder that receives `snake.gif`, `snake.png`, `preview.html` |
| `token`      | The built-in token   | Token for reading public data; keep the default           |

To change the morph, edit `morph` in your copy of the workflow; pushing the change
runs the workflow and redraws the snake. If you change `output-dir`, change the
commit step's path and the README image to match.

Keep `token` at its default. The Action's process holds whatever token the
workflow passes it, so the token bounds what a compromised release could do. The
built-in token reaches only the profile repository, for one job, with the
workflow's permissions; a personal access token would hand over everything it
can reach.

The example uses `flerple08/git-snake@v1`. Each way of referencing the Action
fixes something different:

- `@v1` runs whichever release `v1` points to, and it moves to each compatible
  release.
- A full release tag such as `@v1.0.2` chooses when you update, but anyone who
  can push to this repository can move a tag, so it does not fix what runs.
- A full commit hash fixes `action.yml` and the version it downloads. The binary
  still comes from that version's GitHub Release, and the checksum the Action
  checks it against comes from the same release, so a binary replaced together
  with its checksum would still pass.

GitHub pauses scheduled workflows in a public repository after 60 days without
activity such as a push. A snake that stops changing during a long quiet stretch
adds no commits, so its schedule can pause and does not resume on its own.
Re-enable it from the Actions tab or with `gh workflow enable snake.yml`.

## From the Command Line

Install one of two ways. With Rust at or above the `rust-version` in
[Cargo.toml](../Cargo.toml):

```sh
cargo install git-snake --locked
```

Or download the archive for your platform from the
[releases page](https://github.com/flerple08/git-snake/releases), a `.tar.gz`
for Linux and macOS or a `.zip` for Windows. Each archive has a `.sha256` file
beside it; check the download with `shasum -a 256 -c` on macOS, `sha256sum -c`
on Linux, or `certutil -hashfile` on Windows, then put `git-snake` on your `PATH`.
macOS may refuse to open a binary it downloaded;
`xattr -d com.apple.quarantine git-snake` clears that flag.

Collection goes through the [GitHub CLI](https://cli.github.com), so install it
and sign in once with `gh auth login`. Then:

```sh
git-snake collect octocat --output snapshot.json
git-snake render snapshot.json --output-dir snake --morph banana
```

Open `snake/preview.html` in a browser to see the GIF and the still on GitHub's
light and dark page colors. `--morph` takes `normal`, the default, `pastel`,
`banana`, `albino`, `axanthic`, or `cinnamon`.

`collect` reads only public data: the account's public repositories, the public
repositories it has contributed to, GitHub's color for each language, and its
daily contributions from the account's creation through the last complete UTC
day. It writes the snapshot to standard output, or with `--output` replaces the
file only after collection succeeds. In GitHub Actions, `GH_TOKEN` set to the
workflow's token is enough. `render` draws everything in memory first and
replaces each file through a temporary file, so a failed run keeps the previous
files; a run that succeeds prints nothing. Both print errors to standard error
and exit 1; invalid usage exits 2. `git-snake --help` prints the full usage and
`git-snake --version` the version.

## As a Library

The same crate is a library, documented on [docs.rs](https://docs.rs/git-snake).
Add it beside `serde_json`, which reads the snapshot:

```sh
cargo add git-snake serde_json
```

The pipeline runs one way. Read or collect an `AccountSnapshot`, derive the
snake's development, choose the day's behavior, render frames, and encode them:

```rust
use git_snake::behavior::select_behavior;
use git_snake::coat::SnakeMorph;
use git_snake::history::AccountSnapshot;
use git_snake::snake::derive_snake_development;
use git_snake::motion::frame_count;
use git_snake::output::{encode_gif, encode_png};
use git_snake::render::{render_snake, render_slither_frame};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot_json = std::fs::read_to_string("snapshot.json")?;
    let account_snapshot: AccountSnapshot = serde_json::from_str(&snapshot_json)?;
    let snake_development = derive_snake_development(&account_snapshot)?;
    let snake_morph = SnakeMorph::Banana;
    let behavior = select_behavior(&snake_development);
    let slither_frames: Vec<_> = (0..frame_count(behavior))
        .map(|frame_index| render_slither_frame(&snake_development, snake_morph, behavior, frame_index))
        .collect();
    std::fs::write("snake.gif", encode_gif(&slither_frames)?)?;
    std::fs::write("snake.png", encode_png(&render_snake(&snake_development, snake_morph))?)?;
    Ok(())
}
```

To collect in-process instead of reading a file, call
`collect::collect_account_snapshot` with `collect::GhCli` and today's UTC date.
Everything after collection is deterministic and performs no I/O, so the same
snapshot always draws the same snake. Breaking changes to the API bump the major
version.
