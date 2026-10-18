# Stack-it-16 — Codex Project Instructions

## Project goal
Build a polished, standalone 2D arcade stacking game in Rust. The game should feel as though it could have been released for the Sega Mega Drive / Genesis or Super Nintendo in the early 1990s, while running natively on modern Linux systems.

The project is independent of `kitchen-radio`. In the future, `kitchen-radio` may launch the standalone executable as a separate process. Never introduce a runtime or build dependency on `kitchen-radio` into this repository.

## Canonical visual reference
The **purple/pink/red/orange isometric block tower against a blue sky with clouds** in the user-approved reference image is the visual target. The reference with green/yellow cubes, greenery, or branching block structures is **not** the target.

- Preserve the recognizable core stacking mechanic: a moving horizontal slab aligns over the previous layer, with non-overlapping material trimmed away after placement.
- Render rectangular slabs in a convincing, readable pseudo-isometric perspective; top faces and side faces must be distinguishable.
- Use pixel art, deliberately limited colors, visible dithering where appropriate, crisp nearest-neighbor scaling, and era-inspired UI typography.
- Use a purple-to-pink-to-red-to-orange tower palette, bright blue sky, pixel clouds, and restrained parallax effects.
- Avoid generic cube-building mechanics, branching towers, smooth vector gradients, modern mobile-style UI, or high-resolution anti-aliased art.
- Treat any generated mockup as **art direction**, not as a production-ready sprite sheet or as proof of exact historical hardware constraints.

## Game rules and feel
- One moving block travels horizontally over the top of the existing tower.
- A press/tap places it. Compute overlap with the previous layer; trim the overhang.
- A fully missed placement ends the run.
- Award points for successful placements and a clear bonus for sufficiently accurate placements.
- Increase difficulty progressively while keeping controls responsive, timing predictable, and the game fair.
- Implement the exact positioning, speed progression, tolerance, and scoring values as tunable parameters, and write tests for boundary cases.
- Keep core game simulation deterministic and independent from renderer and input APIs.
- Start with one polished game mode before adding optional modes or features.

## Architecture
- Use a Rust Cargo workspace with at least a pure `stack-core` crate and a runnable `stack-game` crate.
- `stack-core` owns gameplay state, rules, scoring, game-over handling, and deterministic updates. It must not depend on a window or graphics library.
- `stack-game` owns the event loop, rendering, assets, audio, input mapping, menus, and persistence.
- Prefer a small, maintainable dependency set. Evaluate the graphics framework for **both** desktop Linux and Raspberry Pi OS ARM64 support before committing to it; do not assume desktop support guarantees Raspberry Pi compatibility.
- Keep platform-specific code at the edges. Avoid coupling mechanics to framerate: update the simulation using explicit elapsed time or a fixed timestep.
- Start with a logical 320×224 pixel playfield inspired by Mega Drive conventions, but keep viewport fitting and scaling configurable for different displays.
- Prefer nearest-neighbor integer scaling when practical; handle nonmatching aspect ratios without stretching pixels.
- Save highscores and settings using platform-appropriate user-data directories rather than writing into the installed executable's directory.
- Errors should be actionable, especially when a graphical environment, audio device, asset, or display backend is unavailable.

## Target platforms and inputs
- Primary development platform: Nobara/Fedora Linux, x86_64.
- Secondary target: Raspberry Pi 4 running 64-bit Raspberry Pi OS with a 7-inch touchscreen.
- Support keyboard from the first playable build; design an input-action abstraction so touch and gamepad can be added without rewriting game logic.
- Future `kitchen-radio` integration should be possible by launching the executable and returning cleanly to the radio interface on exit.
- Do not change any files in `kitchen-radio` without an explicit separate request.

## Development workflow
- Build the smallest genuinely playable vertical slice first: moving slab, input, overlap, trimming, score, game-over, restart, and temporary pixel-art rendering.
- Then iterate on authentic pixel art, animation, background layers, audio, menus, save data, and additional polish.
- Keep tasks scoped and explain significant architecture decisions before implementing sweeping changes.
- Add unit tests for gameplay rules, including exact alignment, partial overlap, perfect-placement tolerance, no overlap, and min-width edge cases.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` when the workspace is present. If a tool cannot run, state why rather than asserting success.
- Do not commit generated binaries, build artifacts, credentials, or machine-specific settings.
- Update `README.md` when build instructions, controls, features, or dependencies change.
- Keep source code and identifiers in English. User-facing game text should be structured so localization (German and English) can be added cleanly.

## Collaboration expectations
- Keep the initial scope tight; do not invent features or redesign the gameplay without approval.
- Follow the approved purple/red tower image as the art direction whenever drawing or implementing graphics.
- Ask before introducing large dependencies, changing the framework, altering the core gameplay rules, or modifying another repository.
- Separate confirmed working behavior from plans, assumptions, and untested Raspberry Pi claims.
