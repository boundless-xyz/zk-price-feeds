#!/usr/bin/env python3
"""Tiny proxy that translates Pinata v3 API calls to v1 (free plan compatible)."""
import http.server
import json
import sys
import urllib.request
import urllib.error
from io import BytesIO

PINATA_V1_URL = "https://api.pinata.cloud/pinning/pinFileToIPFS"
PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8787


class PinataProxyHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        if "/v3/files" not in self.path:
            self.send_error(404)
            return

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length)
        auth = self.headers.get("Authorization", "")
        content_type = self.headers.get("Content-Type", "")

        # Forward as-is to v1 endpoint (multipart form is compatible)
        req = urllib.request.Request(
            PINATA_V1_URL,
            data=body,
            headers={
                "Authorization": auth,
                "Content-Type": content_type,
            },
        )

        try:
            with urllib.request.urlopen(req) as resp:
                v1_data = json.loads(resp.read())
            # v1 returns: {"IpfsHash": "Qm...", "PinSize": N, ...}
            # v3 expects: {"data": {"id": "...", "cid": "Qm..."}}
            v3_response = {
                "data": {
                    "id": v1_data.get("ID", ""),
                    "cid": v1_data["IpfsHash"],
                }
            }
            response_body = json.dumps(v3_response).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response_body)))
            self.end_headers()
            self.wfile.write(response_body)
            print(f"[proxy] Uploaded -> CID: {v1_data['IpfsHash']}", flush=True)
        except urllib.error.HTTPError as e:
            error_body = e.read()
            self.send_response(e.code)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(error_body)
            print(f"[proxy] Pinata error {e.code}: {error_body.decode()}", flush=True)

    def log_message(self, format, *args):
        pass  # Suppress default access logs


if __name__ == "__main__":
    server = http.server.HTTPServer(("127.0.0.1", PORT), PinataProxyHandler)
    print(f"[proxy] Pinata v3→v1 proxy on http://127.0.0.1:{PORT}", flush=True)
    server.serve_forever()
