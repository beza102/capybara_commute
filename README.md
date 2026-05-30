# 🦫 Capybara Commute

A Bevy/Rust game where you control a capybara floating down a river,
catching falling passengers (ducklings, frogs, butterflies) on your back
— without tipping over!

## Gameplay

- **← / → (or A/D)** to move the capybara left and right
- Catch passengers as they fall from the sky
- Heavier passengers (frogs 🐸) shift your center of mass
- Butterflies 🦋 are light and actually help balance you
- **Topple alert**: if your tilt bar fills up, everyone falls off!
- Stack too many (8+) and you sink — balance quality vs quantity

### Scoring
| Passenger | Weight | Points |
|-----------|--------|--------|
| 🐥 Duckling | 1.0 | 10 |
| 🐸 Frog | 1.8 | 20 |
| 🦋 Butterfly | -0.4 (balances!) | 5 |

## Setup & Run

### Prerequisites
- [Rust](https://rustup.rs/) (stable)
- On Linux: `sudo apt install libasound2-dev libudev-dev pkg-config`
- On macOS: Xcode command line tools (`xcode-select --install`)
- On Windows: Visual Studio C++ build tools

### Run
```bash
git clone <this-repo>
cd capybara_commute
cargo run
```

First compile will take ~2–3 minutes (Bevy is large). Subsequent runs are fast.

### Debug physics outlines
Uncomment this line in `main.rs` to see collision shapes:
```rust
// .add_plugins(RapierDebugRenderPlugin::default())
```

## Architecture (ECS)

| Component | Purpose |
|-----------|---------|
| `Capybara` | Marks the player entity; stores `tilt` and `tilt_velocity` |
| `Passenger` | Falling animals with `kind`, `landed`, `stack_position` |
| `Balanced` | Tag added when a passenger successfully lands |
| `RiverTile` | Scrolling background tiles |
| `GameState` (Resource) | Score, passenger count, spawn timer, difficulty |

### Key Systems
- `move_capybara` — keyboard input → KinematicCharacterController
- `spawn_passengers` — timed spawner with difficulty ramp
- `land_passengers` — collision detection between fallers and capybara back
- `update_tilt` — center-of-mass physics simulation
- `check_game_over` — topple angle or overload detection

## Vibe-Coding Notes

This project was built as a learning exercise in AI-assisted development
with an unfamiliar stack (Rust + Bevy). Some things to observe:

- **Where the agent excels**: boilerplate ECS structure, system wiring,
  component/resource definitions, physics plugin setup
- **Where it struggles**: subtle Bevy API version mismatches, borrow checker
  edge cases in system queries, fine-tuned game feel numbers
- **Key lesson**: commit every time something compiles AND runs correctly.
  Agents can break working code when adding features.

## Git Workflow (recommended)

```bash
# After first successful cargo run:
git init && git add . && git commit -m "MVP: capybara moves, passengers fall"

# After each feature:
git add . && git commit -m "feat: tilt physics working"
git add . && git commit -m "feat: game over + restart"
```
