# Flamingo Agent: Rust Windows Service with Child Process

A background agent written in Rust that, every 5 seconds, collects system metrics and launches a C++ child binary with elevated privileges. The child logs the metrics to stdout and to a log file whose ACL is restricted to **Administrators** and **SYSTEM**.

## Repository layout

```
.
├── agent-rust/            # Rust service (install mode, ACL setup, metrics loop)
│   └── src/
│       ├── main.rs        # CLI: --install / --uninstall / daemon mode
│       ├── service.rs     # Windows service glue + cross-platform metrics loop
│       └── acl.rs         # File permission restriction (Windows ACL / Unix 0600)
├── logger-child/          # C++17 child binary
│   ├── CMakeLists.txt
│   └── src/main.cpp
├── install.ps1            # Build + install + start
├── uninstall.ps1          # Stop + remove service + clean dist/
├── check-service.ps1      # Verify service registration and file ACLs
└── README.md
```

## How it works

1. **`agent-rust`** runs either as a Windows Service (`FlamingoAgent`) or, when not started by the Service Control Manager (or on non-Windows OSes), as a plain background daemon.
2. Every 5 seconds it collects:
   - current UTC time (RFC 3339)
   - memory usage (RSS, in bytes) of the agent process itself, via the cross-platform `sysinfo` crate
3. On startup it creates `metrics.log` next to the executable and applies an ACL to both `metrics.log` and `logger_child.exe`: the Users and Authenticated Users entries are removed and full access is granted to SYSTEM and Administrators.
4. It spawns `logger_child` with two arguments: the metrics string (`UTC=... RSS_BYTES=...`) and the log file path. The service runs as LocalSystem, so the child inherits elevated privileges.
5. The child prints the metrics to stdout and appends a timestamped line to the log file.

**Error handling:** failures in ACL setup, child spawn, or a non-zero child exit are logged to stderr and the loop continues, so the agent doesn't crash. A failed `--install` (for example, the service already exists) is reported as an error instead of a false success.

## Cross-platform notes

- Rust: platform-specific code is isolated with `#[cfg(windows)]` / `#[cfg(not(windows))]`. On non-Windows systems the agent runs in daemon mode and applies `chmod 600` instead of an ACL. `--install` and `--uninstall` are Windows-only.
- C++: standard library only (`<fstream>`, `<chrono>`, ...). The only platform split is `localtime_s` vs `localtime_r`.

## Prerequisites (Windows)

- Windows 10/11 or Server 2019+
- [Rust](https://rustup.rs) (stable, 1.85+ because the crate uses edition 2024)
- Visual Studio Build Tools with the "Desktop development with C++" workload (includes MSVC)
- [CMake](https://cmake.org/download/) 3.14+
- An **elevated** PowerShell (Run as Administrator)

## Build and install

```powershell
# from the repository root, in an elevated PowerShell
Set-ExecutionPolicy -Scope Process Bypass
.\install.ps1
```

The script:
1. Runs `cargo build --release` in `agent-rust/`
2. Builds `logger-child` with CMake (Release)
3. Stops any existing `FlamingoAgent` service and stages `agent-rust.exe` and `logger_child.exe` into `dist/`
4. Runs `dist\agent-rust.exe --install` to register the `FlamingoAgent` service (automatic startup)
5. Starts the service

## Testing

**Check the service and the ACLs:**
```powershell
.\check-service.ps1
```
Expected: `FlamingoAgent Service is INSTALLED` and `PASS` for both `dist\metrics.log` and `dist\logger_child.exe`.

**Watch the log (as Administrator):**
```powershell
Get-Content .\dist\metrics.log -Wait
```
A new line should appear about every 5 seconds, for example:
```
[2026-10-08 12:00:05] UTC=2026-10-08T09:00:05.123+00:00 RSS_BYTES=14811136
```

**Confirm access is denied to normal users:**
```powershell
# from a non-elevated shell
Get-Content .\dist\metrics.log   # -> Access denied
icacls .\dist\metrics.log        # -> only SYSTEM and BUILTIN\Administrators
```

**Run as a plain daemon (no service), in an elevated shell:**
```powershell
cd dist
.\agent-rust.exe
```

**Service manager check:**
```powershell
Get-Service FlamingoAgent        # Status: Running, StartType: Automatic
```

## Uninstall

```powershell
.\uninstall.ps1
```
Stops the service, removes it from the SCM, and deletes `dist/`.

## Known limitations

- The child is launched once per cycle and exits after logging a single entry, so the 5-second cadence is driven by the service.
- The child's stdout isn't visible when the agent runs as a service (services have no console). Use the log file.
- The ACL is applied at agent startup. If `metrics.log` is deleted and recreated externally, restart the service to re-apply it.
- Inheritance is not disabled on the files. Entries inherited from the parent folder (`dist/`) may still apply depending on where the repository is located. Always run `check-service.ps1` to confirm that only SYSTEM and Administrators have access, and install from a location whose parent folder isn't broadly accessible if it reports a FAIL.
- Re-running `install.ps1` when the service already exists prints a "Failed to install" message from `--install` (the service is already registered) and then starts the existing service. Run `uninstall.ps1` first for a clean reinstall.
- `install.ps1` expects the Visual Studio CMake generator (output at `build\Release\`).