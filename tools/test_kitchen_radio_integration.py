#!/usr/bin/env python3
"""Host-side Kitchen-Radio/Stack-it-16 IPC integration test.

The harness uses the real supervisor and includes stack-game's real
``stack-game/src/arcade.rs`` in a temporary Rust probe.  A temporary fake
Kitchen process supplies a launch ticket and records the JSON inbox.  It does
not open either GUI and never touches production state.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path

KITCHEN_ROOT = Path("/home/sysop/Projekte/kitchen-radio")
STACK_ARCADE = Path(__file__).resolve().parents[1] / "stack-game/src/arcade.rs"


def load_supervisor():
    path = KITCHEN_ROOT / "tools/kitchen_radio_arcade_supervisor.py"
    spec = importlib.util.spec_from_file_location("arcade_supervisor", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_stack_probe(path: Path, mode: str = "normal") -> None:
    if mode == "normal":
        body = '''    client.begin_attempt();
    for attempt in 0..3 {
        if attempt != 0 { client.begin_attempt(); }
        for blocks in [20, 40, 60, 80, 100, 110] {
            client.successful_placement(blocks);
        }
        client.finish_attempt(110);
    }'''
    elif mode == "voluntary":
        body = '''    client.begin_attempt();
    client.voluntary_exit();'''
    else:
        raise ValueError(mode)
    path.write_text(
        f'''#[path = {json.dumps(str(STACK_ARCADE))}]
mod arcade;

use arcade::{{ArcadeClient, ArcadeConfig}};
use std::env;

fn main() {{
    let mut client = ArcadeClient::new(ArcadeConfig {{
        socket_path: env::var("STACK_IT_16_ARCADE_SOCKET").unwrap(),
        session_id: env::var("STACK_IT_16_ARCADE_SESSION_ID").unwrap(),
        attempts: env::var("STACK_IT_16_ARCADE_ATTEMPTS").unwrap().parse().unwrap(),
        currency: env::var("STACK_IT_16_ARCADE_CURRENCY").unwrap(),
    }});
{body}
}}
''',
        encoding="utf-8",
    )


def write_crash_probe(path: Path) -> None:
    path.write_text("import sys; raise SystemExit(23)\n", encoding="utf-8")


def write_fake_kitchen(path: Path) -> None:
    path.write_text(
        '''import json
import pathlib
import sys

state = pathlib.Path(sys.argv[1])
launch = state / "arcade-launch.json"
marker = state / "kitchen-ran"
if not marker.exists():
    marker.write_text("1")
    launch.write_text(json.dumps({
        "protocol": 1,
        "type": "launch_stack_it_16",
        "session_id": "kr-integration",
        "attempts": 3,
        "currency": "rings",
        "executable": None,
        "created_at_unix": 4102444800,
    }) + "\\n")
else:
    rows = [json.loads(line) for line in (state / "arcade-events.jsonl").read_text().splitlines()]
    (state / "observed.json").write_text(json.dumps(rows))
''',
        encoding="utf-8",
    )


def supervisor_args(root: Path, fake_kitchen: Path, stack_command: str | Path):
    return type("Args", (), {
        "state_dir": root,
        "kitchen_command": f"{sys.executable} {fake_kitchen} {root}",
        "stack_command": str(stack_command),
    })()


def compile_probe(source: Path, binary: Path, mode: str) -> None:
    write_stack_probe(source, mode)
    subprocess.run(
        ["rustc", "--edition", "2021", "-O", str(source), "-o", str(binary)],
        check=True,
    )


def assert_complete_attempts(rows: list[dict]) -> None:
    types = [row["type"] for row in rows]
    assert types.count("session_started") == 1
    assert types.count("attempt_started") == 3
    assert types.count("attempt_finished") == 3
    assert types.count("session_completed") == 1
    milestones = [row for row in rows if row["type"] == "milestone"]
    assert len(milestones) == 18
    assert sum(row["reward"] for row in milestones) == 330
    assert len({row["idempotency_key"] for row in milestones}) == 18


def main() -> None:
    supervisor = load_supervisor()
    with tempfile.TemporaryDirectory(prefix="stack-it-16-integration-") as raw:
        root = Path(raw)
        fake_kitchen = root / "fake_kitchen.py"
        write_fake_kitchen(fake_kitchen)

        normal_source = root / "stack_probe.rs"
        normal_binary = root / "stack_probe"
        compile_probe(normal_source, normal_binary, "normal")
        result = supervisor.run_sequence(supervisor_args(root, fake_kitchen, normal_binary))
        assert result == 0, result
        rows = json.loads((root / "observed.json").read_text())
        assert_complete_attempts(rows)
        assert not (root / "arcade-launch.json").exists()

        crash_root = root / "crash-recovery"
        crash_root.mkdir()
        crash_kitchen = crash_root / "fake_kitchen.py"
        write_fake_kitchen(crash_kitchen)
        crash_probe = crash_root / "crash.py"
        write_crash_probe(crash_probe)
        result = supervisor.run_sequence(
            supervisor_args(crash_root, crash_kitchen, f"{sys.executable} {crash_probe}")
        )
        assert result == 23
        assert (crash_root / "arcade-launch.json").exists()
        recovery_source = crash_root / "recovery.rs"
        recovery_binary = crash_root / "recovery"
        compile_probe(recovery_source, recovery_binary, "normal")
        result = supervisor.run_sequence(supervisor_args(crash_root, crash_kitchen, recovery_binary))
        assert result == 0
        assert_complete_attempts(json.loads((crash_root / "observed.json").read_text()))
        assert not (crash_root / "arcade-launch.json").exists()

        voluntary_root = root / "voluntary"
        voluntary_root.mkdir()
        voluntary_kitchen = voluntary_root / "fake_kitchen.py"
        write_fake_kitchen(voluntary_kitchen)
        voluntary_source = voluntary_root / "voluntary.rs"
        voluntary_binary = voluntary_root / "voluntary"
        compile_probe(voluntary_source, voluntary_binary, "voluntary")
        result = supervisor.run_sequence(
            supervisor_args(voluntary_root, voluntary_kitchen, voluntary_binary)
        )
        assert result == 0
        voluntary_rows = json.loads((voluntary_root / "observed.json").read_text())
        assert [row["type"] for row in voluntary_rows] == [
            "session_started", "attempt_started", "voluntary_exit"
        ]

        bridge_root = root / "validation"
        bridge = supervisor.EventBridge(
            bridge_root / "events.sock", bridge_root / "events.jsonl", "kr-validation"
        )
        valid = {
            "protocol": 1,
            "type": "milestone",
            "session_id": "kr-validation",
            "attempt_id": 1,
            "blocks": 20,
            "currency": "rings",
            "reward": 20,
            "idempotency_key": "kr-validation:1:20",
        }
        assert bridge.accept(valid)
        assert bridge.accept(valid)  # duplicate delivery is transport-safe
        assert not bridge.accept({**valid, "session_id": "other"})
        assert not bridge.accept({**valid, "currency": "yen"})
        assert bridge.accept({**valid, "currency": "bananas"})  # importer rejects purchase mismatch
        assert bridge.accept({**valid, "reward": 999})  # importer rejects invalid amount

        print("normal: 26 events, 330 reward units")
        print("crash/recovery: launch request retained then replayed")
        print("voluntary exit: completion event omitted")
        print("duplicate/session/currency transport validation: OK")


if __name__ == "__main__":
    main()
