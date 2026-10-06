"""Unix PTY smoke check. Run after cargo build: python3 tests/terminal_smoke.py."""
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path


def read_until(master, needle, timeout=5):
    output = b""
    deadline = time.monotonic() + timeout
    while needle not in output and time.monotonic() < deadline:
        if select.select([master], [], [], 0.05)[0]:
            output += os.read(master, 65536)
    assert needle in output, (needle, output)
    return output


def main():
    binary = Path(__file__).resolve().parents[1] / "target/debug/precision"
    with tempfile.TemporaryDirectory() as directory:
        master, slave = pty.openpty()
        process = None
        try:
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
            original = termios.tcgetattr(slave)
            process = subprocess.Popen(
                [str(binary), "--db", directory + "/workout.db", "tui"],
                stdin=slave, stdout=slave, stderr=slave,
            )
            read_until(master, b"F10 Quit")
            os.write(master, b"\r")
            read_until(master, b"Draft ID: 1")
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
            process.send_signal(signal.SIGWINCH)
            read_until(master, b"F10 Quit")
            os.write(master, b"\x03")
            read_until(master, b"\x1b[?1049l")
            assert process.wait(timeout=5) == 0
            assert termios.tcgetattr(slave) == original
            saved = subprocess.check_output(
                [str(binary), "--db", directory + "/workout.db", "workout", "show", "1", "--json"]
            )
            assert b'"state": "draft"' in saved
            print("Terminal smoke passed: keyboard, resize, retained draft, restoration")
        finally:
            if process is not None and process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
