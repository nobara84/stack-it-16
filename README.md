# Stack-it-16

Stack-it-16 ist ein kleines, eigenständiges 16-Bit-inspiriertes
Stacking-Arcade-Spiel in Rust. Ein beweglicher isometrischer Block muss auf
dem vorherigen Block platziert werden; Überstände werden abgeschnitten.

## Features

- Isometrisches Block-Stacking mit Cutting-Mechanik
- Perfekte Platzierungen und höhenabhängige Blockfarben
- Pixel-Art-Fantasiehimmel mit vertikaler Parallax-Welt
- Cinematisches Zeitlupen-Death-Replay
- Lokaler Highscore und deutsche Benutzeroberfläche
- Tastatur-, Maus- und Touch-Steuerung

## Technik und Zielplattformen

- Rust und Macroquad
- Logische Auflösung: **320×224 Pixel**
- Linux-Desktop als primäre Plattform
- Raspberry Pi 4 als Zielplattform; dort ist die Laufzeit noch nicht
  vollständig verifiziert
- MIT-Lizenz

## Bauen und starten

Auf Nobara/Fedora werden die Macroquad-Systembibliotheken benötigt:

```bash
sudo dnf install pkg-config libX11-devel libXi-devel mesa-libGL-devel alsa-lib-devel
```

Im Projektverzeichnis:

```bash
cargo build --release -p stack-game
cargo run --release -p stack-game
```

Das fertige Programm liegt danach unter
`target/release/stack-game`. Die Fenstergröße startet standardmäßig mit
3× 320×224 und verwendet bei Größenänderung Integer-Skalierung mit
Letterboxing.

## Steuerung

- Leertaste, Enter, Mausklick oder Touch: Starten / Block platzieren
- Escape: Pause fortsetzen
- R: nach Game Over neu starten
- Leertaste, Enter, Mausklick oder Touch: Death Replay überspringen
- F3: Diagnose-Overlay ein-/ausschalten

## Arcade-Modus

Wird das Spiel vom Kitchen-Radio gestartet, aktiviert es sich über die vier
dokumentierten Umgebungsvariablen `STACK_IT_16_ARCADE_SOCKET`,
`STACK_IT_16_ARCADE_SESSION_ID`, `STACK_IT_16_ARCADE_ATTEMPTS=3` und
`STACK_IT_16_ARCADE_CURRENCY` (`rings` oder `bananas`). Die Kommunikation
erfolgt als zeilengetrenntes JSON über den Unix-Socket des Radio-Bridge-
Prozesses. Ohne diese Variablen bleibt das Spiel vollständig standalone und
liest keine Kitchen-Radio-Dateien.

Im Arcade-Game-Over erscheinen Touch-Schaltflächen für den nächsten Versuch
oder die Rückkehr zum Radio. Milestones werden nur für tatsächlich platzierte
Blöcke und mit einem stabilen Idempotenzschlüssel gemeldet.

## Entwicklungsstatus

Der spielbare Kern, Highscore, Kamera-Follow, Parallax-Hintergrund,
Falling-Fragmente und Death Replay sind implementiert. Die visuelle Prüfung
auf Nobara ist noch der maßgebliche nächste Schritt; Raspberry-Pi-4-
Performance und Grafik-Backend sind noch nicht abschließend getestet.

## Screenshot

Im Repository ist derzeit kein echter Gameplay-Screenshot abgelegt. Daher wird
hier bewusst kein Mockup als Laufzeit-Screenshot ausgegeben.

## Entwicklung

Kurze Build-Historie: [CHANGELOG.md](CHANGELOG.md)

Arcade-Deployment und Host-Integrationstest: [Kitchen-Radio-Arcade-Deployment](docs/kitchen-radio-arcade-deployment.md)

Für Änderungen gelten insbesondere:

```bash
cargo fmt --all --check
cargo test --workspace --all-targets --release
cargo build --release
git diff --check
```
