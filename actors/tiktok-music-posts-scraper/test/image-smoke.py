#!/usr/bin/env python3
import json
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


class SmokeHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, _format, *_args):
        pass

    def send_json(self, status, value):
        body = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)

    def read_json(self):
        return json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))

    def do_GET(self):
        path = urlsplit(self.path).path
        if path.startswith("/api/"):
            if self.headers.get("X-API-Key") != "smoke-scrappa-key":
                self.send_json(401, {"message": "invalid Scrappa key"})
                return
            query = parse_qs(urlsplit(self.path).query)
            if path != "/api/tiktok/music/posts" or query != {
                "music_id": ["7001"],
                "count": ["2"],
                "cursor": ["0"],
            }:
                self.send_json(400, {"message": "unexpected Scrappa request"})
                return
            self.send_json(200, {
                "code": 0,
                "data": {
                    "posts": [{"aweme_id": "smoke-post", "desc": "Image smoke"}],
                    "hasMore": True,
                    "cursor": "next-page",
                },
                "processed_time": 12,
            })
            return

        if self.headers.get("Authorization") != "Bearer smoke-apify-token":
            self.send_json(401, {"message": "invalid Apify token"})
            return
        if path == "/v2/key-value-stores/store-smoke/records/INPUT":
            self.send_json(200, {"musicIds": ["7001"], "count": 2, "cursor": "0"})
        elif path == "/v2/actor-runs/run-smoke":
            self.send_json(200, {
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {
                            "actorChargeEvents": {
                                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            },
                        },
                    },
                    "options": {"maxTotalChargeUsd": 1},
                    "chargedEventCounts": {},
                },
            })
        else:
            self.send_json(404, {"message": "unexpected GET path", "path": path})

    def do_POST(self):
        if self.headers.get("Authorization") != "Bearer smoke-apify-token":
            self.send_json(401, {"message": "invalid Apify token"})
            return
        if urlsplit(self.path).path != "/v2/datasets/dataset-smoke/items":
            self.send_json(404, {"message": "unexpected dataset path"})
            return
        self.server.dataset_rows = self.read_json()
        self.send_json(201, {})

    def do_PUT(self):
        if self.headers.get("Authorization") != "Bearer smoke-apify-token":
            self.send_json(401, {"message": "invalid Apify token"})
            return
        if urlsplit(self.path).path != "/v2/key-value-stores/store-smoke/records/OUTPUT":
            self.send_json(404, {"message": "unexpected output path"})
            return
        self.server.output = self.read_json()
        self.send_json(200, {})


def main():
    image = sys.argv[1] if len(sys.argv) > 1 else "tiktok-music-posts-scraper"
    server = ThreadingHTTPServer(("0.0.0.0", 0), SmokeHandler)
    server.dataset_rows = None
    server.output = None
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base_url = f"http://127.0.0.1:{server.server_port}"
    command = [
        "docker",
        "run",
        "--rm",
        "--network",
        "host",
        "--env",
        f"APIFY_API_PUBLIC_BASE_URL={base_url}",
        "--env",
        f"SCRAPPA_API_BASE_URL={base_url}/api",
        "--env",
        "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=store-smoke",
        "--env",
        "ACTOR_DEFAULT_DATASET_ID=dataset-smoke",
        "--env",
        "ACTOR_RUN_ID=run-smoke",
        "--env",
        "APIFY_TOKEN=smoke-apify-token",
        "--env",
        "SCRAPPA_API_KEY=smoke-scrappa-key",
        image,
    ]
    try:
        run = subprocess.run(
            command,
            capture_output=True,
            text=True,
            check=False,
            timeout=90,
        )
        if run.returncode != 0:
            raise RuntimeError(
                f"Actor container exited {run.returncode}.\n{run.stdout}{run.stderr}"
            )
        if server.dataset_rows != [{
            "aweme_id": "smoke-post",
            "desc": "Image smoke",
            "request_music_id": "7001",
        }]:
            raise RuntimeError(f"Unexpected dataset rows: {server.dataset_rows!r}")
        if server.output is None or server.output["posts_extracted"] != 1:
            raise RuntimeError(f"Unexpected OUTPUT record: {server.output!r}")
        if server.output["results"][0]["next_cursor"] != "next-page":
            raise RuntimeError(f"Pagination cursor missing from OUTPUT: {server.output!r}")
        print("Local actor image smoke passed: dataset and OUTPUT writes matched.")
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


if __name__ == "__main__":
    main()
