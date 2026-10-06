"""Real terminal delivery, resize, disconnect recovery, and raw-mode restoration."""
import fcntl
import os
import select
import signal
import struct
import subprocess
import sys
import tempfile
from pathlib import Path
import termios
import time

temporary = None
if len(sys.argv) == 1:
    temporary = tempfile.TemporaryDirectory()
    binary = str(Path(__file__).resolve().parents[1] / "target/debug/precision")
    database = temporary.name + "/terminal.db"
else:
    binary, database = sys.argv[1:]
master, slave = os.openpty()
original = termios.tcgetattr(slave)
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
process = subprocess.Popen([binary, "--db", database, "tui"], stdin=slave, stdout=slave, stderr=slave)
received = b""

def until(text):
    global received
    deadline = time.monotonic() + 8
    target = text.encode()
    received = b""
    while target not in received:
        assert time.monotonic() < deadline, (text, received.decode(errors="replace"))
        if select.select([master], [], [], 0.1)[0]:
            received += os.read(master, 65536)
        assert process.poll() is None, received.decode(errors="replace")

try:
    until("Start a draft")
    os.write(master, b"\x1bOQ")  # F2
    until("Draft ID: 1")
    os.write(master, b"\x1b[15~")  # F5
    until("Exercise search")
    os.write(master, b"SQUAT\r")
    until("Attempted repetitions")
    os.write(master, b"\t\t\t0")
    until("Attempted repetitions: 0")
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    os.kill(process.pid, signal.SIGWINCH)
    until("Attempted repetitions: 0")
    os.write(master, b"\x1b[24~")  # F12
    until("0 attempted repetitions")
    # Ctrl-C is delivered as a key in raw mode, so the RAII guard restores state.
    os.write(master, b"\x03")
    process.wait(timeout=8)
    assert process.returncode == 0
    assert termios.tcgetattr(slave) == original, "raw terminal attributes were not restored"
    tail = b""
    while select.select([master], [], [], 0.1)[0]:
        tail += os.read(master, 65536)
    assert b"\x1b[?1049l" in tail, "alternate screen was not restored"
    exported = subprocess.check_output([binary, "--db", database, "workout", "show", "1", "--json", "--actual-only"])
    import json
    assert json.loads(exported)["sets"][0]["portions"][0]["repetitions"] == 0
finally:
    if process.poll() is None:
        process.kill()
        process.wait()
    os.close(master)
    os.close(slave)
    if temporary is not None:
        temporary.cleanup()
