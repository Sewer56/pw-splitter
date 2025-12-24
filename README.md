# pw-splitter

A TUI application for PipeWire that lets you capture audio from an application at full volume for recording while independently controlling your local listening volume.

## The Problem

When you're recording or streaming with OBS, you often want to:
- Send game/application audio to OBS at **full volume** for a clean recording
- Listen to that same audio locally at a **lower volume** so it doesn't blast your ears
    - e.g. You want people talking to be louder than game.

In PipeWire, adjusting an application's volume affects **all destinations**.
If you turn down a game's volume to save your hearing, your recording also gets quieter.

## The Solution

`pw-splitter` creates two parallel audio paths from your source application - one to your recording software at full volume, and one to your speakers with adjustable volume.

BEFORE (volume affects everything):

```
    [Game Audio] ──────────────────────────────> [Speakers]
         │                                        (loud!)
         │
         └─────────────────────────────────────> [OBS Recording]
                                                  (also loud!)

    Turning down the game makes BOTH quieter.
```

AFTER (independent volume control):
```
    [Game Audio] ───┬──> [To Recording] ──> [OBS Mic/Aux]
                    │     (full volume)
                    │
                    └──> [To Local] ──> [Speakers]
                          (adjustable)

    Adjust "To Local" volume in pwvucontrol - recording stays at 100%.
```

## Installation

**First, check your package manager** (e.g., `dnf install pw-splitter`, `pacman -S pw-splitter`) for a pre-packaged version.

### From GitHub Releases (Recommended)

Download the latest release from [GitHub Releases](https://github.com/Sewer56/pw-splitter/releases). Pre-built binaries are available for:

- `linux-x64.zip` (x86_64)
- `linux-x86.zip` (i686)
- `linux-arm64.zip` (aarch64)

```bash
# Download and run (example for x86_64)
wget https://github.com/Sewer56/pw-splitter/releases/latest/download/linux-x64.zip
unzip linux-x64.zip
chmod +x pw-splitter
./pw-splitter
```

### From crates.io

If you have Rust installed, install from crates.io:

```bash
cargo install pw-splitter
```

## Quick Start

```bash
# Move to source directory
cd src

# Build and run the project
cargo run --release

# Use arrow keys to select:
#   1. Source application (e.g., "Dolphin Emulator")
#   2. Recording destination (e.g., "OBS [Mic/Aux]")
#   3. Press Enter to confirm

# Adjust local volume in pwvucontrol
# Look for the loopback with "Local" in the name
```

## Usage

### Interactive Mode (TUI)

```bash
pw-splitter
```

| Key                | Action           |
| ------------------ | ---------------- |
| `↑`/`↓` or `j`/`k` | Navigate list    |
| `Enter`            | Select / Confirm |
| `Esc`              | Go back          |
| `r`                | Refresh list     |
| `q`                | Quit             |

### Command Line

```bash
pw-splitter list        # Show active splits
pw-splitter stop <name> # Stop a specific split
pw-splitter stop-all    # Stop all splits
```

## How It Looks in qpwgraph

After setting up a split for `Dolphin Emulator` to `OBS [Mic/Aux]`:

```
┌────────────────────────────────────────────┐
│ Dolphin Emulator [Dolphin Audio Output]    │  ◄── Source application
└──────────┬─────────────────────┬───────────┘
           │                     │
           ▼                     ▼
┌────────────────────┐    ┌────────────────────┐
│ Dolphin Emulator   │    │ Dolphin Emulator   │
│ -> Local           │    │ -> OBS             │
│ [...Local input]   │    │ [...OBS input]     │  ◄── Loopback capture sides
└─────────┬──────────┘    └──────────┬─────────┘
          │                          │
          ▼                          ▼
┌────────────────────┐    ┌────────────────────┐
│ Dolphin Emulator   │    │ Dolphin Emulator   │
│ -> Local           │    │ -> OBS             │
│ [...Local output]  │    │ [...OBS output]    │  ◄── Loopback playback sides
└─────────┬──────────┘    └──────────┬─────────┘
          │                          │
          ▼                          ▼
┌────────────────────┐    ┌────────────────────┐
│ USB Audio Speakers │    │ OBS-1 [Mic/Aux]    │
└────────────────────┘    └────────────────────┘
```

### Nodes created by pw-splitter:

| Node Name                                | Type                | Purpose                                     |
| ---------------------------------------- | ------------------- | ------------------------------------------- |
| `Dolphin Emulator -> Local [... input]`  | Loopback (capture)  | Captures from source app                    |
| `Dolphin Emulator -> Local [... output]` | Loopback (playback) | Sends to speakers - **adjust this volume!** |
| `Dolphin Emulator -> OBS [... input]`    | Loopback (capture)  | Captures from source app                    |
| `Dolphin Emulator -> OBS [... output]`   | Loopback (playback) | Sends to OBS at full volume                 |

### To adjust local volume:

In **pwvucontrol** or **qpwgraph**, find the loopback with "Local" in the name and adjust its volume.
The recording to OBS will stay at 100% regardless of this setting.

---

# Technical Details

## PipeWire Terminology

| Term                    | Meaning                                                                         |
| ----------------------- | ------------------------------------------------------------------------------- |
| **Sink**                | An audio destination (like speakers). Applications "sink" their audio into it.  |
| **Source**              | An audio origin (like a microphone). Applications capture audio from it.        |
| **Node**                | Any audio endpoint in PipeWire - could be an app, device, or virtual component. |
| **Loopback**            | Takes audio from one place and sends it to another. Like an audio cable.        |
| **Stream/Output/Audio** | An application playing audio (e.g., game, music player).                        |
| **Stream/Input/Audio**  | An application recording audio (e.g., OBS audio capture).                       |

## Architecture

The split works by creating two **pw-loopback** instances that both capture from the source application:

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────┐
│    Source    │     │  pw-loopback    │     │   Speakers   │
│  Application │────▶│  (to Local)     │────▶│              │
│              │     │  [adjustable]   │     │              │
│              │     └─────────────────┘     └──────────────┘
│              │
│              │     ┌─────────────────┐     ┌──────────────┐
│              │     │  pw-loopback    │     │     OBS      │
│              │────▶│  (to Recording) │────▶│  [Mic/Aux]   │
│              │     │  [full volume]  │     │              │
└──────────────┘     └─────────────────┘     └──────────────┘
```

### Step-by-Step Process

1. **Create two loopbacks** with `node.autoconnect=false` on both capture and playback sides:

   ```bash
   # Recording loopback (to OBS)
   pw-loopback \
     --capture-props='media.class=Audio/Sink node.name=MyApp_to_Recording node.description="MyApp -> OBS" node.autoconnect=false stream.capture.sink=true' \
     --playback-props='media.class=Stream/Output/Audio node.name=MyApp_to_Recording node.description="MyApp -> OBS" node.autoconnect=false'

   # Local loopback (to speakers)
   pw-loopback \
     --capture-props='media.class=Audio/Sink node.name=MyApp_to_Local node.description="MyApp -> Local" node.autoconnect=false stream.capture.sink=true' \
     --playback-props='media.class=Stream/Output/Audio node.name=MyApp_to_Local node.description="MyApp -> Local" node.autoconnect=false'
   ```

   Key options:
   - `node.autoconnect=false` - Prevents PipeWire from auto-connecting
   - `stream.capture.sink=true` - Captures from sinks (apps), not sources (mics)

2. **Disconnect source from original outputs**:

   ```bash
   # Remove existing links to speakers
   pw-link -d "Dolphin Emulator:output_FL" "speakers:playback_FL"
   pw-link -d "Dolphin Emulator:output_FR" "speakers:playback_FR"

   # Also remove any existing links to the recording destination
   pw-link -d "Dolphin Emulator:output_FL" "OBS:input_FL"
   pw-link -d "Dolphin Emulator:output_FR" "OBS:input_FR"
   ```

3. **Connect source to both loopback capture inputs**:

   ```bash
   # Source → Recording loopback capture
   pw-link "Dolphin Emulator:output_FL" "MyApp_to_Recording:input_FL"
   pw-link "Dolphin Emulator:output_FR" "MyApp_to_Recording:input_FR"

   # Source → Local loopback capture
   pw-link "Dolphin Emulator:output_FL" "MyApp_to_Local:input_FL"
   pw-link "Dolphin Emulator:output_FR" "MyApp_to_Local:input_FR"
   ```

4. **Connect loopback playback outputs to destinations**:

   ```bash
   # Recording loopback → OBS input (using port IDs to avoid node name ambiguity)
   pw-link 85 118   # port ID 85 = loopback output_FL, port ID 118 = OBS input_FL
   pw-link 86 119   # port ID 86 = loopback output_FR, port ID 119 = OBS input_FR

   # Local loopback → Speakers (can use names since speakers have unique names)
   pw-link "MyApp_to_Local:output_FL" "speakers:playback_FL"
   pw-link "MyApp_to_Local:output_FR" "speakers:playback_FR"
   ```

   Note: Port IDs are used for OBS because multiple OBS inputs share `node.name="OBS"`,
   making port names like `OBS:input_FL` ambiguous.

### Why Manual Linking for OBS?

OBS audio inputs are `Stream/Input/Audio` nodes - they're **capture streams** that read FROM sinks, not sinks themselves. You can't target them with pw-loopback's `target.object` on the playback side.

Additionally, multiple OBS inputs share the same `node.name` ("OBS"), making port names like `OBS:input_FL` ambiguous. We use **port IDs** directly to ensure we connect to the correct OBS input.

### State Management

Active splits are stored in `/tmp/pw-splitter/<name>.json`:

```json
{
  "name": "DolphinEmulator_Split",
  "source_node_id": 158,
  "recording_loopback_name": "DolphinEmulator_to_Recording",
  "local_loopback_name": "DolphinEmulator_to_Local",
  "recording_dest_node_id": 118,
  "loopback_to_recording_pid": 12345,
  "loopback_to_local_pid": 12346,
  "original_links": [
    {"output_port": "Dolphin Emulator:output_FL", "input_port": "speakers:playback_FL"}
  ]
}
```

This enables:
- Listing active splits
- Proper cleanup (kill processes, restore original links)
- Auto-restart of crashed loopback processes

### Cleanup Process

When stopping a split:
1. Kill both `pw-loopback` processes
2. Restore original audio links
3. Delete the state file

## Building

```bash
cargo build --release
```

Dependencies:
- `ratatui` - TUI framework
- `crossterm` - Terminal handling
- `serde` / `serde_json` - State serialization
- `argh` - CLI parsing
- `thiserror` - Error handling

Runtime requirements:
- PipeWire
- `pw-link`, `pw-loopback`, `pw-dump` commands

## License

MIT

I needed something easy to use for a one-off stream; and maybe future use.
Given that an LLM did most of the heavy lifting, I'd rather release this for
absolutely free.

## Developer Manual

For step-by-step development guidance, see the [Developer Manual](https://reloaded-project.github.io/reloaded-templates-rust/manual/).

## Contributing

We welcome contributions! See the [Contributing Guide](https://reloaded-project.github.io/reloaded-templates-rust/manual/#contributing) for details.