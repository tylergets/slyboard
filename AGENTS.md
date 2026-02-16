# Agent Instructions for `slyboard`

## Environment
- Use `nix develop` to enter the project dev shell.
- `cargo` is available from within `nix develop`.

## Required Workflow
- When you create a new file, always stage it immediately with `git add <path>`.
- Whenever you change the configuration file (`slyboard.yaml`), also update `nix/home-manager.nix` and `README.md` to keep docs and module behavior aligned.
- Run `nix build` after changes that can affect the actual program behavior or build output.

## Validation
- Treat `nix build` as the required project-level verification step before finishing work when changes can affect the actual program. For docs-only changes (for example `README.md` and other documentation text), `nix build` is not required.

## Deployment
- When asked to deploy or push, update the version in `Cargo.toml` and `flake.nix`.
- Create a commit message and push to the main branch.