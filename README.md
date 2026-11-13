# Paper Minimap 0.2.0

A native Rust mod for **Horizon**, with a vanilla-style paper minimap, an explored-world atlas, and waypoints.

**Development build: source implemented; Horizon compilation and in-game validation are still required.** The .hnmod contains source, not a precompiled game. See [VALIDATION.txt](VALIDATION.txt) for the checks actually performed. A passing core test run does not establish compatibility with the game.

## Features and controls

- North-up terrain map using the game's map-color palette and loaded vanilla player-arrow sprite. Original pixel parchment border; no Minecraft assets included.
- **M** opens the full-screen map. The automatic default becomes **J** if another registered vanilla/native control uses M. Explicit user rebindings take precedence. If the selected key also conflicts, the map refuses to open and asks you to rebind **Full map** in **Options > Mods > Paper Minimap**.
- **Mouse wheel**, **+ / -**, or toolbar buttons zoom between 0.25 and 32 blocks per GUI pixel. Zoom follows the cursor. **Left drag** or **arrow keys** pan; **Home / Center** returns to the player.
- **Right-click the map** to add a waypoint there, or **Add point** to use your position. **Waypoints** opens the list; click an entry to edit its name, X/Y/Z, color, icon, visibility, and world-marker permission.
- In an editor, **Tab / Shift+Tab** changes fields, **Backspace** deletes the last character, **Ctrl+A** clears the active field, **Enter** saves, and **Escape** returns to the map. Delete requires a second click.
- **M/J**, **Escape**, or **Close** closes the atlas. While editing a name, M and J type normally.
- Terrain begins as blank parchment and fills only inside the configured exploration radius as you move. Panning and zooming do not reveal terrain. Only already-loaded chunks are sampled.
- Exploration and waypoints persist separately for each world/server and dimension. Previously explored terrain stays visible after its chunks unload.
- Waypoints appear **on the map by default**. Physical world markers require BOTH **Show waypoints in world** and each waypoint's **Allow in-world marker** switch. Both start off. World markers are camera-facing, depth-tested labels; they do not render through walls.

The full-screen map uses Horizon's native mod screen to release the mouse and stop movement input. Like an inventory, it does **not** pause the world.

## Settings

Open **Settings** on the map toolbar, or use **Options > Mods > Paper Minimap > Settings**. Click an option to cycle values; scroll the map's settings panel for additional rows.

| Setting | Values / default |
| --- | --- |
| Minimap visibility | On by default |
| Minimap size | 64, 96, **128**, 160, 192, 256 GUI pixels |
| Corner | All four; **top right** |
| Minimap zoom | **1**, 2, 4, 8, 16 blocks/pixel |
| Map controls/text size | 75, **100**, 125, 150, 200%; fits small windows |
| Paper border | 0, 3, 5, **7**, 10 pixels |
| Minimap opacity | 40, 60, 80, **100**% |
| Map waypoint size | 4, 6, **8**, 10, 12, 16 pixels |
| Names, distances, label backgrounds | Independent switches, on by default |
| Map waypoint opacity | 40, 60, 80, **100**% |
| Physical world markers | **Off** |
| World marker distance | 64, 128, 256, **512**, 1024, 2048 blocks |
| World marker size | 50, 75, **100**, 150, 200% |
| World marker opacity | 25, 50, **75**, 100% |
| World marker height offset | 1, **2**, 4, 8, 16 blocks |
| Exploration radius | 16, 24, **32**, 48, 64 blocks |
| Sampling work per tick | 32, 64, **128**, 256, 512 columns |
| Exploration autosave | 5, **10**, 30, 60 seconds |
| Center on player when opening | On by default |
| North indicator | On by default |

Waypoint colors: gold, red, green, blue, purple, orange, cyan, white. Icons: diamond, pin, cross, square. Up to 256 waypoints per world/dimension.

## Build and integrate with Horizon

This bundle targets Minecraft **26.3**, Horizon Nightly **2026-11-08** at
`fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`,
native hooks **411**. The manifest and compatibility.json pin the same commit.
Direct terrain/world-render access is version-sensitive; newer Nightlies need a fresh integration check.

Use the official [make-a-mod guide](https://playhorizon.dev/docs/make-a-mod) and [test-a-mod guide](https://playhorizon.dev/docs/test-a-mod).

1. Prepare the desired instance in Horizon and run it once.
2. From PowerShell in this project folder, invoke the installed launcher:

```powershell
& "$env:LOCALAPPDATA\Programs\Horizon Launcher\horizon-launcher.exe" native-build "$PWD\mixins\paper_minimap" --trust-native-code --instance horizon
```

The launcher manages the Rust toolchain and builds the mod into the selected instance's game. Use your actual executable path/instance name if different. The supplied Build.ps1 wraps the same command; direct invocation works without changing PowerShell script policy.

3. When the native build succeeds, press **Play**, enter a world, and enable Paper Minimap in **Options > Mods**.
4. Press **M** (or the automatic **J** fallback), explore, and add a waypoint.

For the first test, use an instance with no other mod uploading to Horizon's shared native HUD image slot. This implementation uses that slot for its atlas; simultaneous image providers need coordination.

## Validation and packaging

Run the standalone logic tests with stable Rust (edition 2024):

```sh
cargo test --locked
python check_bundle.py --package
```

The Python 3.11+ command checks bundle references and metadata, writes the deterministic source archive into dist/, and reads it back byte-for-byte. Package.ps1 is an equivalent packaging option when scripts are allowed.

For the full official integration check, use a Linux/WSL development environment with Rust, Python 3.11+, rsync, and Horizon's native build dependencies:

```sh
git clone https://github.com/horizon-rust/horizon-mods
cd horizon-mods
python3 native/tools/native.py check /absolute/path/to/horizon-minimap
python3 native/tools/native.py package /absolute/path/to/horizon-minimap
```

GitHub Actions runs the core tests and bundle checks on pushes. **Horizon integration** is a separate manually dispatched workflow that applies the bundle and runs the official game compile check. Neither substitutes for the in-game checklist below.

## Saved data and implementation

Data lives under the game directory's `horizon-mods/.horizon-mod-data/paper_minimap/`.
Settings are individual files. `atlas-v2/<identity-hash>/` contains an identity record, 32x32 terrain tiles, and waypoints.txt. Server identity is the connection address; different saves behind the same address cannot be distinguished automatically. Back up this directory to preserve maps.

Writes use a temporary file and rename. Invalid tiles/waypoint files are preserved rather than silently replaced. Failed tile saves remain dirty for retry, including after a world switch. The cache holds up to 1,024 terrain tiles; older tiles are saved before eviction. Unknown regions do not allocate tile caches or request chunks. The atlas texture is capped at 512x512 and updates at most 10 times/second.

- mixins/paper_minimap/runtime.rs: settings, viewport math, disk storage, waypoint editor state.
- mixins/paper_minimap/adapter.rs: Horizon input, sampling, GUI, and world labels.
- mixins/paper_minimap/horizon-mixin.toml and helpers: hook registration.
- tests/core.rs: behavior tests, including failed saves and corrupt data.

Overworld/End use surface colors; under the Nether roof, sampling starts near the player's height. This is an explored surface atlas, not a cave scanner. Remote unvisited terrain and entities are not revealed.

## In-game acceptance checklist

- M opens; bind another control to M and verify automatic J. Bind J too and verify the conflict notice. Test a custom map binding and typing M/J in a waypoint name.
- Zoom around the cursor; pan far into blank terrain; verify no new terrain appears until you travel there.
- Explore across negative coordinates and chunk borders; leave/rejoin and restart Horizon; verify explored terrain persists.
- Change dimensions/worlds/servers and verify terrain, editor actions, and waypoints never carry across.
- Create/edit/delete named waypoints. Confirm world markers start off and need both opt-ins. Test distance cutoff, size/opacity, and terrain occlusion.
- Resize the window, change game GUI scale, cycle every setting, open chat/inventory, toggle F1/F3, disable/re-enable the mod, and return to the title screen.
- Check logs for panics, image-slot conflicts, and save failures. Measure frame time while exploring and zooming out.

License: MIT. No Minecraft textures or game files are distributed.
