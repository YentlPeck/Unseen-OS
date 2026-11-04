"""Serve the Unseen OS site on localhost and open it in the browser.

Double-click (or run `python serve.py`). Keeps running until closed.
"""

import functools
import http.server
import os
import socketserver
import threading
import webbrowser

PORT = 4173
SITE = os.path.dirname(os.path.abspath(__file__))
URL = f"http://127.0.0.1:{PORT}/"


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def main():
    handler = functools.partial(Quiet, directory=SITE)
    socketserver.TCPServer.allow_reuse_address = True
    try:
        server = socketserver.TCPServer(("127.0.0.1", PORT), handler)
    except OSError:
        webbrowser.open(URL)  # already running: just open it
        return
    threading.Timer(0.3, webbrowser.open, [URL]).start()
    print(f"Unseen OS site: {URL}  (close this window to stop)")
    server.serve_forever()


if __name__ == "__main__":
    main()
