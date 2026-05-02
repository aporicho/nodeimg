# Launch scripts

Developer mode is enabled by the Rust debug profile. User mode uses the release profile.
Diagnostic mode also uses the debug profile, enables project trace logs, and writes to a dedicated diagnostic log file.

`*_build_run.sh` calls `cargo run`, so it compiles if needed and then starts the app.
`*_no_build_run.sh` starts the existing binary directly from `target/` and fails if it is missing.

## macOS

| Mode | Compile if needed | No compile |
| --- | --- | --- |
| User | `scripts/mac_user_build_run.sh` | `scripts/mac_user_no_build_run.sh` |
| Developer | `scripts/mac_dev_build_run.sh` | `scripts/mac_dev_no_build_run.sh` |
| Diagnostic | `scripts/mac_diag_build_run.sh` | `scripts/mac_diag_no_build_run.sh` |

## Windows

The Windows scripts are WSL/bash launchers that run the app on the Windows side through PowerShell.

| Mode | Compile if needed | No compile |
| --- | --- | --- |
| User | `scripts/windows_user_build_run.sh` | `scripts/windows_user_no_build_run.sh` |
| Developer | `scripts/windows_dev_build_run.sh` | `scripts/windows_dev_no_build_run.sh` |
| Diagnostic | `scripts/windows_diag_build_run.sh` | `scripts/windows_diag_no_build_run.sh` |

## Compatibility wrappers

- `scripts/mac_run.sh` -> `scripts/mac_user_build_run.sh`
- `scripts/mac_run.sh --dev` -> `scripts/mac_dev_build_run.sh`
- `scripts/mac_run.sh --diag` -> `scripts/mac_diag_build_run.sh`
- `scripts/windows_run.sh` -> `scripts/windows_user_build_run.sh`
- `scripts/windows_run.sh --dev` -> `scripts/windows_dev_build_run.sh`
- `scripts/windows_run.sh --diag` -> `scripts/windows_diag_build_run.sh`

Use `RUST_LOG=...` or `LOG_DIR=...` to override defaults.
