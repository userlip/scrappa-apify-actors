#!/usr/bin/env python3
import json
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


class SmokeState:
    input = {"queries": ["Berlin"], "limit": 1}
    dataset = []
    output = None
    scrappa_requests = []


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def send_json(self, status, value):
        body = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def read_json(self):
        return json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))

    def do_GET(self):
        path = urlsplit(self.path)
        if path.path == "/v2/actor-runs/smoke-run":
            self.send_json(200, {"data": {
                "pricingInfo": {"pricingModel": "FREE"},
                "chargedEventCounts": {},
                "options": {},
            }})
            return
        if path.path == "/v2/key-value-stores/smoke-kv/records/INPUT":
            self.send_json(200, SmokeState.input)
            return
        if path.path == "/api/immobilienscout24/locations":
            query = parse_qs(path.query)
            SmokeState.scrappa_requests.append((query.get("query", [None])[0], query.get("limit", [None])[0]))
            if self.headers.get("X-API-Key") != "local-image-smoke-key":
                self.send_json(403, {"message": "missing test API key"})
                return
            self.send_json(200, {"locations": [{
                "geocode": "1276003001",
                "name": "Berlin",
                "type": "city",
            }]})
            return
        self.send_json(404, {"message": "unexpected GET " + self.path})

    def do_POST(self):
        if self.path == "/v2/datasets/smoke-dataset/items":
            SmokeState.dataset.extend(self.read_json())
            self.send_json(201, {})
            return
        self.send_json(404, {"message": "unexpected POST " + self.path})

    def do_PUT(self):
        if self.path == "/v2/key-value-stores/smoke-kv/records/OUTPUT":
            SmokeState.output = self.read_json()
            self.send_json(201, {})
            return
        self.send_json(404, {"message": "unexpected PUT " + self.path})


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 test/image-smoke.py IMAGE")

    SmokeState.dataset = []
    SmokeState.output = None
    SmokeState.scrappa_requests = []
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    api_base = f"http://127.0.0.1:{server.server_port}/"

    try:
        result = subprocess.run([
            "docker", "run", "--rm", "--network", "host",
            "--env", f"APIFY_API_PUBLIC_BASE_URL={api_base}",
            "--env", f"SCRAPPA_API_BASE_URL={api_base}api/",
            "--env", "APIFY_TOKEN=local-image-smoke-token",
            "--env", "ACTOR_RUN_ID=smoke-run",
            "--env", "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=smoke-kv",
            "--env", "ACTOR_DEFAULT_DATASET_ID=smoke-dataset",
            "--env", "SCRAPPA_API_KEY=local-image-smoke-key",
            sys.argv[1],
        ], capture_output=True, text=True)
    finally:
        server.shutdown()
        thread.join()
        server.server_close()

    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)

    expected = [{
        "geocode": "1276003001",
        "name": "Berlin",
        "type": "city",
        "source_query": "Berlin",
    }]
    assert SmokeState.scrappa_requests == [("Berlin", "1")], SmokeState.scrappa_requests
    assert SmokeState.dataset == expected, SmokeState.dataset
    assert SmokeState.output == expected, SmokeState.output
    print("local image smoke passed: input, authenticated Scrappa request, dataset, and OUTPUT KV")


if __name__ == "__main__":
    main()
