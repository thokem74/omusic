#!/usr/bin/env python3
"""Run inside dbus-run-session; exercise the actual TUI, mpv, and MPRIS."""

import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time


def run(binary):
    with tempfile.TemporaryDirectory(prefix="omusic-smoke-") as temporary:
        root = Path(temporary)
        music = root / "music" / "Album"
        music.mkdir(parents=True)
        runtime = root / "run"
        runtime.mkdir(mode=0o700)
        for number, title in [(1, "First"), (2, "Last")]:
            subprocess.run(
                ["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                 "sine=frequency=440:duration=8", "-metadata", f"title={title}",
                 "-metadata", "artist=Aster", "-metadata", "album_artist=Aster",
                 "-metadata", "album=Signals", "-metadata", f"track={number}",
                 str(music / f"{number}.mp3")], check=True,
            )
        env = dict(os.environ, TERM="xterm-256color", XDG_STATE_HOME=str(root / "state"),
                   XDG_CONFIG_HOME=str(root / "config"), XDG_RUNTIME_DIR=str(runtime))
        stale_events = os.environ.get("OMUSIC_TEST_STALE_EVENTS") == "1"
        if stale_events:
            helpers = root / "helpers"
            helpers.mkdir()
            (helpers / "mpv").symlink_to(Path("tests/support/fake_mpv.py").resolve())
            env["PATH"] = str(helpers) + os.pathsep + env["PATH"]
        master, slave = pty.openpty()
        slave_path = os.ttyname(slave)
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
        process = subprocess.Popen([binary, "--root", str(music.parent), "--audio-null"],
                                   stdin=slave, stdout=slave, stderr=slave, env=env,
                                   start_new_session=True)
        os.close(slave)
        output = bytearray()

        def drain():
            while select.select([master], [], [], 0)[0]:
                try:
                    data = os.read(master, 65536)
                except OSError:
                    break
                if not data:
                    break
                output.extend(data)
                if b"\x1b[6n" in data:
                    os.write(master, b"\x1b[1;1R")

        def wait(condition, description):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                drain()
                if condition():
                    return
                assert process.poll() is None, output.decode(errors="replace")
                time.sleep(0.04)
            raise AssertionError(f"Timed out: {description}\n" + output.decode(errors="replace"))

        def bus(*args):
            return subprocess.run(["busctl", "--user", *args], env=env, capture_output=True, text=True)

        def property_value(name):
            return bus("get-property", "org.mpris.MediaPlayer2.omusic", "/org/mpris/MediaPlayer2",
                       "org.mpris.MediaPlayer2.Player", name).stdout

        def call(name, *args):
            result = bus("call", "org.mpris.MediaPlayer2.omusic", "/org/mpris/MediaPlayer2",
                         "org.mpris.MediaPlayer2.Player", name, *args)
            assert result.returncode == 0, result.stderr

        try:
            wait(lambda: b"First" in output and b"Last" in output, "scanned tracks in TUI")
            assert "Stopped" in property_value("PlaybackStatus")
            os.write(master, b"\r\r\r")
            wait(lambda: "Playing" in property_value("PlaybackStatus"), "keyboard playback")
            wait(lambda: "First" in property_value("Metadata"), "desktop track metadata")
            if stale_events:
                call("Next")
                wait(lambda: "Last" in property_value("Metadata"), "replacement track selected")
                time.sleep(0.4)
                assert "Playing" in property_value("PlaybackStatus"), "Old EOF must not stop the replacement track"
                os.write(master, b"q")
                process.wait(timeout=5)
                assert process.returncode == 0
                print("PASS: delayed EOF is not attributed to the replacement track")
                return
            call("Pause")
            wait(lambda: "Paused" in property_value("PlaybackStatus"), "MPRIS pause")
            call("Next")
            wait(lambda: "Last" in property_value("Metadata"), "MPRIS next")
            call("Play")
            wait(lambda: "Playing" in property_value("PlaybackStatus"), "MPRIS play")
            before = property_value("Position")
            call("Seek", "x", "1000000")
            wait(lambda: property_value("Position") != before, "MPRIS seek")
            repeat_launch = subprocess.run([binary], env=env, capture_output=True, text=True, timeout=5)
            assert repeat_launch.returncode == 0 and "already running" in repeat_launch.stdout
            os.write(master, b"?")
            wait(lambda: b"Controls" in output or b"help" in output, "help")
            os.write(master, b"\x1b")
            fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 20, 60, 0, 0))
            process.send_signal(signal.SIGWINCH)
            os.write(master, b"Q")
            wait(lambda: b"reorder" in output, "queue view")
            os.write(master, b"C")
            wait(lambda: "First" not in property_value("Metadata"), "clear queue")
            os.write(master, b"q")
            process.wait(timeout=5)
            assert process.returncode == 0, output.decode(errors="replace")
            assert not (runtime / "omusic" / "mpv.sock").exists()
            assert bus("status", "org.mpris.MediaPlayer2.omusic").returncode != 0
            # Reopen through the real binary, verify it remains stopped, then SIGHUP.
            reopened = os.open(slave_path, os.O_RDWR)
            process = subprocess.Popen([binary, "--audio-null"], stdin=reopened, stdout=reopened,
                                       stderr=reopened, env=env, start_new_session=True)
            os.close(reopened)
            wait(lambda: "Stopped" in property_value("PlaybackStatus"), "stopped restoration")
            process.send_signal(signal.SIGHUP)
            process.wait(timeout=5)
            assert process.returncode == 0
            assert not (runtime / "omusic" / "mpv.sock").exists()
            print("PASS: terminal browsing, mpv playback, MPRIS metadata/controls, queue, resize, single instance, exit and SIGHUP cleanup")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
            os.close(master)


if __name__ == "__main__":
    run(str(Path(sys.argv[1]).resolve()))
