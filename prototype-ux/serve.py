"""Throwaway UX comparison. Run: python3 prototype-ux/serve.py"""

from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


if __name__ == "__main__":
    handler = partial(SimpleHTTPRequestHandler, directory=str(Path(__file__).parent))
    server = ThreadingHTTPServer(("127.0.0.1", 8765), handler)
    print("Omusic UX prototypes: http://127.0.0.1:8765/?variant=A", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.server_close()
