# Background assets

The game loads `sky_world.png` as the 320×224 pixel-art far-sky scene and
applies nearest-neighbor sampling. It is drawn once as the continuous far
backdrop; transparent scenery cards provide the moving world layers. The
scene was generated from the approved blue-sky / floating-cliff art direction
because the original reference file was not available in the project
workspace.

Optional transparent 320×224 overlays are supported by the renderer:

- `distant_clouds.png` — parallax factor 0.24
- `floating_cliffs.png` — parallax factor 0.48
- `foreground_clouds.png` — parallax factor 0.78

The renderer places these transparent layers in deterministic vertical world
bands. A new foreground band arrives about every twelve tower levels, while the atmosphere color
blends through authored height regions at levels 0, 15, 30, 45, 60, 75, 90,
105 and beyond. These temporary layers are derived from the base image and
are intentionally replaceable with hand-authored art. Missing overlays are
logged and do not stop the game; the composed fallback remains visible.
