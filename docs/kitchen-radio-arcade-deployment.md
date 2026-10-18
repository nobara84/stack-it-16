# Stack-it-16-Arcade-Deployment auf dem Raspberry Pi

Diese Anleitung beschreibt den geplanten Betrieb mit Kitchen-Radio auf einem
Raspberry Pi 4 / Raspberry Pi OS Lite ARM64. Sie wurde auf Nobara vorbereitet,
nicht auf echter Pi-Hardware abgenommen. Vorhandene User-Units, Shop-Daten und
Assets dürfen erst nach einer Sicherung ersetzt werden.

## 1. Vor dem Update sichern

Auf dem Pi als `kitchen-radio`:

```sh
systemctl --user cat kitchen-radio.service > /tmp/kitchen-radio.service.before
cp /home/kitchen-radio/start-radio.sh /tmp/start-radio.sh.before
cp -a "$HOME/.local/state/kitchen-radio" \
  "$HOME/.local/state/kitchen-radio.backup.$(date +%Y%m%d-%H%M%S)"
```

Die laufende Unit und das Startskript nicht durch ein geratenes Kiosk-Rezept
ersetzen. Vorhandene Seat-, Rotation-, Audio- und Asset-Argumente bleiben
maßgeblich.

## 2. ARM64-Builds

Für beide Projekte auf dem Pi die vorhandene Rust-Toolchain verwenden. Ein
x86_64-Nobara-Binary ist nicht für ARM64 geeignet.

```sh
cd /home/kitchen-radio/Projekte/kitchen-radio
cargo build --release --locked --target aarch64-unknown-linux-gnu -j 1

cd /home/kitchen-radio/Projekte/stack-it-16
cargo build --release --target aarch64-unknown-linux-gnu -j 1
```

Vor dem Umschalten prüfen:

```sh
file /home/kitchen-radio/Projekte/kitchen-radio/target/aarch64-unknown-linux-gnu/release/kitchen-radio
file /home/kitchen-radio/Projekte/stack-it-16/target/aarch64-unknown-linux-gnu/release/stack-game
```

Die Stack-it-16-Assets müssen neben dem Binary beziehungsweise relativ zum
Arbeitsverzeichnis des Spiels vorhanden sein. Der Supervisor injiziert die vier
Arcade-Variablen; sie werden nicht dauerhaft in eine Shell- oder Service-Datei
geschrieben.

## 3. Host-Test vor der Umstellung

Auf Nobara im Stack-it-16-Checkout:

```sh
python3 tools/test_kitchen_radio_integration.py
```

Der Test verwendet einen temporären State-Ordner, einen lokalen Unix-Socket,
den echten Supervisor und den echten Stack-it-16-IPC-Code. Er öffnet keine GUI
und verändert keine Shop- oder Wallet-Dateien.

## 4. Supervisor-Aufruf

Der produktive Cage-Prozess soll den sequenziellen Supervisor als einzigen
Startprozess ausführen:

```sh
dbus-run-session cage -- python3 \
  /home/kitchen-radio/Projekte/kitchen-radio/tools/kitchen_radio_arcade_supervisor.py \
  --state-dir /home/kitchen-radio/.local/state/kitchen-radio \
  --kitchen-command \
  '/home/kitchen-radio/Projekte/kitchen-radio/target/aarch64-unknown-linux-gnu/release/kitchen-radio --fullscreen --assets /home/kitchen-radio/Projekte/kitchen-radio/extracted' \
  --stack-command \
  '/home/kitchen-radio/Projekte/stack-it-16/target/aarch64-unknown-linux-gnu/release/stack-game'
```

Die tatsächlichen absoluten Pfade müssen mit
`systemctl --user cat kitchen-radio.service` und dem vorhandenen
`start-radio.sh` abgeglichen werden. Nicht gleichzeitig eine zweite Cage- oder
Wayland-Instanz starten.

## 5. Abnahme und Rollback

Nach dem Start nacheinander prüfen:

1. Radio startet und Touch funktioniert.
2. Shop-Kauf für 20 Ringe und für 20 Bananen jeweils in getrennten Sessions.
3. Stack-it-16 erhält drei Versuche; `arcade-events.jsonl` enthält bestätigte
   JSON-Ereignisse.
4. Nach Rückkehr werden Wallet-Gutschriften importiert und doppelte
   Idempotenzschlüssel nur einmal angerechnet.
5. Spielabbruch und Radio-Neustart lassen eine gültige Launch-Anforderung
   innerhalb des 15-Minuten-Fensters bestehen.

Bei Problemen zuerst die laufende User-Unit stoppen, die vorherige Unit und das
Startskript wiederherstellen und den gesicherten State-Ordner zurückkopieren.
Keine `shop.json` während einer laufenden Session manuell bearbeiten. Die
genauen Service- und Backup-Pfade des Geräts sind vor der Installation privat
zu dokumentieren.

## Offene Hardware-Abnahme

Noch nicht auf dem Pi bestätigt sind ARM64-Builddauer, Cage/KMS-Start mit der
vorhandenen User-Unit, Touch-Koordinaten im Stack-it-16-Fenster, Audio-Mixing,
langes Parallax-Rendering und Verhalten bei SIGTERM/Power-Cycle. Für diese
Prüfungen ist ein lokaler Test nicht ausreichend.
