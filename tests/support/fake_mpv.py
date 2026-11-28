#!/usr/bin/env python3
"""Protocol fixture: delay the old EOF until the next load command."""
import json
import socket
import sys

path = next(arg.split("=", 1)[1] for arg in sys.argv if arg.startswith("--input-ipc-server="))
server = socket.socket(socket.AF_UNIX)
server.bind(path)
server.listen(1)
connection, _ = server.accept()
stream = connection.makefile("rw")
entry = 0


def emit(message):
    stream.write(json.dumps(message) + "\n")
    stream.flush()


for line in stream:
    request = json.loads(line)
    command = request["command"]
    if command[0] == "loadfile":
        if entry:
            emit({"event": "end-file", "reason": "eof", "playlist_entry_id": entry})
        emit({"request_id": request.get("request_id", 0), "error": "success"})
        entry += 1
        emit({"event": "start-file", "playlist_entry_id": entry})
        emit({"event": "file-loaded"})
    else:
        emit({"request_id": request.get("request_id", 0), "error": "success"})
