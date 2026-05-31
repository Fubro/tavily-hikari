import http.client
import json
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.request import urlopen

EDGEONE_MOCK_URL = os.environ.get("EDGEONE_MOCK_URL", "http://edgeone-mock:9000")
PORT = int(os.environ.get("INGRESS_PORT", "8080"))


def current_origin():
    with urlopen(f"{EDGEONE_MOCK_URL}/origin", timeout=2) as response:
        payload = json.loads(response.read().decode("utf-8"))
    origin = payload["origin"]
    host, port = origin.rsplit(":", 1)
    return host, int(port)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        print("[edgeone-ingress]", fmt % args, flush=True)

    def _proxy(self):
        host, port = current_origin()
        length = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(length) if length else None
        headers = {k: v for k, v in self.headers.items() if k.lower() != "host"}
        conn = http.client.HTTPConnection(host, port, timeout=10)
        conn.request(self.command, self.path, body=body, headers=headers)
        response = conn.getresponse()
        data = response.read()
        self.send_response(response.status)
        for key, value in response.getheaders():
            if key.lower() in {"connection", "transfer-encoding", "content-length"}:
                continue
            self.send_header(key, value)
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self._proxy()

    def do_POST(self):
        self._proxy()

    def do_PUT(self):
        self._proxy()

    def do_PATCH(self):
        self._proxy()

    def do_DELETE(self):
        self._proxy()


ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()
