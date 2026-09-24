#!/usr/bin/env python3
"""Run the production image against local Apify and Scrappa HTTP mocks."""

import argparse
import json
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("image", help="production image tag to run")
    image = parser.parse_args().image
    state = {"requests": [], "dataset_batches": [], "charges": [], "output": None}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, _format, *_args):
            pass

        def respond(self, status, body):
            encoded = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            self.end_headers()
            self.wfile.write(encoded)

        def record(self):
            length = int(self.headers.get("Content-Length", "0"))
            body = self.rfile.read(length) if length else b""
            state["requests"].append(
                {
                    "method": self.command,
                    "path": self.path,
                    "headers": {key.lower(): value for key, value in self.headers.items()},
                    "body": body,
                }
            )
            return body

        def do_GET(self):
            self.record()
            path = urlsplit(self.path).path
            if path == "/v2/actor-runs/test-run":
                self.respond(
                    200,
                    {
                        "data": {
                            "pricingInfo": {
                                "pricingModel": "PAY_PER_EVENT",
                                "pricingPerEvent": {
                                    "actorChargeEvents": {
                                        "challenge-result": {"eventPriceUsd": 0.00025}
                                    }
                                },
                            },
                            "options": {"maxTotalChargeUsd": 1.0},
                            "chargedEventCounts": {},
                        }
                    },
                )
                return
            if path == "/v2/key-value-stores/test-store/records/INPUT":
                self.respond(200, {"keywords": ["cosplay", "fitness"], "count": 2})
                return
            if path == "/api/tiktok/challenges/search":
                query = parse_qs(urlsplit(self.path).query)
                keyword = query.get("keywords", [""])[0]
                if keyword == "cosplay":
                    challenges = [
                        {"id": "1001", "cha_name": "cosplay", "desc": "Costume videos", "stats": {"view_count": 100}},
                        {"challenge_id": "1002", "challenge_name": "cosplaylook", "video_count": 20},
                    ]
                elif keyword == "fitness":
                    challenges = [{"cid": "2001", "name": "fitness", "desc": "Training videos"}]
                else:
                    self.respond(400, {"message": "unexpected keyword"})
                    return
                self.respond(200, {"code": 0, "processed_time": 4, "data": {"challenges": challenges}})
                return
            self.respond(404, {"message": "unexpected GET"})

        def do_POST(self):
            body = self.record()
            if self.path == "/v2/datasets/test-dataset/items":
                state["dataset_batches"].append(json.loads(body))
                self.respond(201, {})
                return
            if self.path == "/v2/actor-runs/test-run/charge":
                state["charges"].append(json.loads(body))
                self.respond(201, {})
                return
            self.respond(404, {"message": "unexpected POST"})

        def do_PUT(self):
            body = self.record()
            if self.path == "/v2/key-value-stores/test-store/records/OUTPUT":
                state["output"] = json.loads(body)
                self.respond(201, {})
                return
            self.respond(404, {"message": "unexpected PUT"})

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    base_url = f"http://127.0.0.1:{server.server_port}"
    environment = {
        "SCRAPPA_API_KEY": "local-smoke-key",
        "APIFY_TOKEN": "local-smoke-token",
        "ACTOR_DEFAULT_KEY_VALUE_STORE_ID": "test-store",
        "ACTOR_DEFAULT_DATASET_ID": "test-dataset",
        "ACTOR_RUN_ID": "test-run",
        "APIFY_API_PUBLIC_BASE_URL": base_url,
        "SCRAPPA_API_BASE_URL": f"{base_url}/api",
    }
    try:
        result = subprocess.run(
            ["docker", "run", "--rm", "--network", "host", *sum((["-e", f"{key}={value}"] for key, value in environment.items()), []), image],
            capture_output=True,
            text=True,
            timeout=45,
            check=False,
        )
    finally:
        server.shutdown()
        server.server_close()
        server_thread.join(timeout=5)

    if result.returncode != 0:
        print(result.stdout)
        print(result.stderr)
        raise SystemExit(f"local image exited with status {result.returncode}")

    dataset = [item for batch in state["dataset_batches"] for item in batch]
    assert len(dataset) == 3, dataset
    assert dataset[0]["challenge_id"] == "1001", dataset[0]
    assert dataset[0]["challenge_name"] == "cosplay", dataset[0]
    assert dataset[0]["request_keyword"] == "cosplay", dataset[0]
    assert dataset[0]["view_count"] == 100, dataset[0]
    assert [charge["eventName"] for charge in state["charges"]] == ["challenge-result"] * 2
    assert [charge["count"] for charge in state["charges"]] == [2, 1]
    assert state["output"]["keywords_requested"] == 2, state["output"]
    assert state["output"]["keywords_completed"] == 2, state["output"]
    assert state["output"]["challenges_extracted"] == 3, state["output"]

    upstream = [request for request in state["requests"] if request["path"].startswith("/api/")]
    assert len(upstream) == 2, upstream
    assert all(request["headers"].get("x-api-key") == "local-smoke-key" for request in upstream)
    assert all("count=2" in request["path"] for request in upstream)

    print("Local image smoke passed: 2 upstream requests, 3 dataset rows, 3 challenge-result charges, OUTPUT written.")


if __name__ == "__main__":
    main()
