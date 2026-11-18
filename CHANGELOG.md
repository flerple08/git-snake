# Changelog

Notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Forked from [git-koi](https://github.com/yalongwastaken/git-koi) and redrawn as a ball
  python. The koi became a snake: a spine-based body with a rounded head, eyes, and a
  forked tongue; six morphs (`normal`, `pastel`, `banana`, `albino`, `axanthic`,
  `cinnamon`) replace the six koi bodies, and the `--body` option and Action input
  are now `--morph` and `morph`. Languages repaint the blotches along its flanks.
  The current streak lights a dorsal stripe instead of the fins.
- Behaviors: resting and deep rest coil into a ball, surface feeding became `alert`
  (stretched out, tongue flicking), and the lap swims became slithers.

### Added

- The Action runs on Windows runners, downloading and checking the release's
  `.zip` (#36).

### Changed

- `git-snake render` prints nothing when it succeeds. The line it printed named
  the behavior, date, and morph, which `preview.html` already shows (#43).

### Fixed

- When GitHub's contribution calendar leaves out a day or reports one twice,
  `git-snake collect` says so and asks to try again, instead of naming a repair
  only the author of a snapshot file can make (#56).

## [1.0.2] - 2026-10-31

### Fixed

- `git-snake collect` requests again any contribution days GitHub leaves out of an
  answer, instead of failing with "missing contribution date". GitHub left out
  days for some accounts whose history crosses a February 29 (#52).

## [1.0.1] - 2026-10-28

### Changed

- The Action is listed as `git-profile-snake`, since a GitHub user already holds
  the name `git-snake`, and its description fits the Marketplace limit.

## [1.0.0] - 2026-10-28

### Added

- `git-snake collect`, which gathers an account's public snapshot through the
  GitHub CLI, and `--help` and `--version` on the command.
- `git-snake render`, which draws the day's snake as a looping GIF, a still PNG, and
  a preview page, in six morphs.
- A GitHub Action that downloads the release binary and runs both inside a
  profile repository's workflow.
- The `git-snake` library: account history, snake development, behavior, motion,
  markings, rendering, and encoding.

[Unreleased]: https://github.com/flerple08/git-snake/compare/v1.0.2...HEAD
[1.0.2]: https://github.com/flerple08/git-snake/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/flerple08/git-snake/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/flerple08/git-snake/releases/tag/v1.0.0
