# wstrace CLI Reference Manual

Comprehensive command, option, and keyboard shortcut reference for `wstrace`.

---

## Synopsis

```text
wstrace [OPTIONS] <COMMAND>
```

---

## Commands

### `run`
Launches the specified executable under active trace monitoring.

```text
wstrace run <EXECUTABLE_PATH> [-- <ARGUMENTS>...]
```

**Examples:**
```powershell
# Interactive tracing
wstrace run notepad.exe

# Tracing with program arguments
wstrace run cmd.exe -- /c dir

# Recursively trace child processes
wstrace -C run parent_script.exe
```

---

### `attach`
Attaches monitoring to an already running target process without restarting it.

```text
wstrace attach [-p <PID>] [-n <NAME>]
```

**Examples:**
```powershell
# Attach by Process ID
wstrace attach -p 4820

# Attach by executable name
wstrace attach -n msedge.exe
```

---

### `all`
Monitors the entire Windows system in real time, capturing all active processes, new process spawns, exits, and module activities.

```text
wstrace all
```

**Examples:**
```powershell
# Interactive system-wide trace
wstrace all

# Stream system-wide activity to console for 30 seconds
wstrace -m stream -d 30 all

# System-wide trace filtering for failures only
wstrace -f all
```

---

## Global Options

| Option | Long Flag | Description |
| :--- | :--- | :--- |
| `-m <MODE>` | `--mode` | Output mode: `tui` (interactive interface) or `stream` (stdout stream). Default: `tui`. |
| `-f` | `--only-failures` | Filter exclusively for failed operations (e.g. access denied, not found). |
| `-C` | `--children` | Recursively monitor child processes spawned by target. |
| | `--include <STR>` | Include only targets/operations containing this substring. |
| | `--exclude <STR>` | Exclude targets/operations containing this substring. |
| `-c <CAT>` | `--category` | Monitored event category: `file`, `reg`, `net`, `proc`, `mem`, or `all`. Default: `all`. |
| `-d <SEC>` | `--duration` | Maximum tracing duration in seconds. |
| | `--export-json <FILE>` | Export captured events to formatted JSON upon exit. |
| | `--copy` | Automatically copy all captured events to clipboard upon exit. |
| `-h` | `--help` | Print help information. |
| `-V` | `--version` | Print version information. |

---

## TUI Keyboard Shortcuts

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| `Ctrl+C` | Copy Selected | Copies highlighted event details into the Windows clipboard. |
| `Ctrl+A` | Copy All | Copies all currently filtered events into the Windows clipboard. |
| `Space` / `Ctrl+E` | Pause / Resume | Freezes the event display without dropping incoming telemetry. |
| `Ctrl+X` | Clear Buffer | Empties the current event list from display memory. |
| `Tab` | Cycle Category | Filters events: `ALL` -> `FILE` -> `REG` -> `NET` -> `PROC`. |
| `f` | Failures Only | Toggles display of failed operations only. |
| `Up` / `Down` (or `j`/`k`) | Scroll | Navigates the event list for detailed inspection. |
| `q` / `Esc` | Quit | Terminates the tracing session cleanly. |
