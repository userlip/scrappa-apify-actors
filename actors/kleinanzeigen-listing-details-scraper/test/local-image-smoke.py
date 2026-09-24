#!/usr/bin/env python3
import json
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

IMAGE = "kleinanzeigen-listing-details-scraper:local-smoke"
REMOVED_ID = "3451021120"
REQUESTED_ID = "3451021121"


class SmokeState:
    def __init__(self):
        self.dataset_items = []
        self.output = None
        self.charges = []
        self.requests = []
        self.lock = threading.Lock()


class Handler(BaseHTTPRequestHandler):
    state = None

    def log_message(self, *_args):
        pass

    def _body(self):
        length = int(self.headers.get("Content-Length", "0"))
        return self.rfile.read(length) if length else b""

    def _json_response(self, status, body):
        encoded = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def _record(self, method, body=None):
        with self.state.lock:
            self.state.requests.append((method, self.path, body))

    def do_GET(self):
        parsed = urlparse(self.path)
        self._record("GET")
        if parsed.path == "/v2/actor-runs/run-123":
            return self._json_response(
                200,
                {
                    "data": {
                        "pricingInfo": {
                            "pricingModel": "PAY_PER_EVENT",
                            "pricingPerEvent": {
                                "actorChargeEvents": {
                                    "listing-detail-result": {"eventPriceUsd": 0.00025},
                                    "apify-default-dataset-item": {"eventPriceUsd": 0.0},
                                    "apify-actor-start": {"eventPriceUsd": 0.0},
                                }
                            },
                        },
                        "options": {"maxTotalChargeUsd": 0},
                        "chargedEventCounts": {"apify-actor-start": 1},
                    }
                },
            )
        if parsed.path == "/v2/key-value-stores/store-123/records/INPUT":
            return self._json_response(200, {"query": "fahrrad"})
        if parsed.path == "/api/kleinanzeigen/search":
            assert self.headers.get("X-API-Key") == "smoke-key"
            assert parse_qs(parsed.query) == {"query": ["fahrrad"], "page": ["1"]}
            return self._json_response(
                200,
                {"data": [{"id": REMOVED_ID}, {"id": REQUESTED_ID}]},
            )
        if parsed.path == "/api/kleinanzeigen/details":
            assert self.headers.get("X-API-Key") == "smoke-key"
            assert self.headers.get("Accept") == "application/json"
            assert self.headers.get("User-Agent") == (
                "thescrappa-kleinanzeigen-listing-details-scraper/1.0"
            )
            ad_id = parse_qs(parsed.query)["ad_id"][0]
            if ad_id == REMOVED_ID:
                return self._json_response(404, {"message": "removed"})
            assert ad_id == REQUESTED_ID
            return self._json_response(
                200,
                {"data": {"id": REQUESTED_ID, "title": "Local image smoke"}},
            )
        self._json_response(404, {"message": "Unexpected GET path"})

    def do_POST(self):
        body = json.loads(self._body())
        self._record("POST", body)
        if self.path == "/v2/actor-runs/run-123/charge":
            assert self.headers.get("Authorization") == "Bearer smoke-token"
            assert self.headers.get("Idempotency-Key") == "run-123-listing-detail-result-1"
            assert body == {"eventName": "listing-detail-result", "count": 1}
            self.state.charges.append(body)
            return self._json_response(201, {})
        if self.path == "/v2/datasets/dataset-123/items":
            assert self.headers.get("Authorization") == "Bearer smoke-token"
            self.state.dataset_items.append(body)
            return self._json_response(201, {})
        self._json_response(404, {"message": "Unexpected POST path"})

    def do_PUT(self):
        body = json.loads(self._body())
        self._record("PUT", body)
        if self.path == "/v2/key-value-stores/store-123/records/OUTPUT":
            assert self.headers.get("Authorization") == "Bearer smoke-token"
            self.state.output = body
            return self._json_response(201, {})
        if self.path == "/v2/actor-runs/run-123":
            return self._json_response(200, {"data": body})
        self._json_response(404, {"message": "Unexpected PUT path"})


def main():
    state = SmokeState()
    Handler.state = state
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    port = server.server_address[1]

    try:
        subprocess.run(
            [
                "docker",
                "build",
                "-f",
                ".actor/Dockerfile",
                "-t",
                IMAGE,
                ".",
            ],
            check=True,
        )
        subprocess.run(
            [
                "docker",
                "run",
                "--rm",
                "--network",
                "host",
                "-e",
                f"APIFY_API_PUBLIC_BASE_URL=http://127.0.0.1:{port}",
                "-e",
                f"SCRAPPA_API_BASE_URL=http://127.0.0.1:{port}/api",
                "-e",
                "SCRAPPA_API_KEY=smoke-key",
                "-e",
                "APIFY_TOKEN=smoke-token",
                "-e",
                "ACTOR_RUN_ID=run-123",
                "-e",
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=store-123",
                "-e",
                "ACTOR_DEFAULT_DATASET_ID=dataset-123",
                "-e",
                "ACTOR_INPUT_KEY=INPUT",
                IMAGE,
            ],
            check=True,
        )

        assert len(state.dataset_items) == 1, state.dataset_items
        assert state.dataset_items[0]["id"] == REQUESTED_ID
        assert state.dataset_items[0]["request_ad_id"] == REQUESTED_ID
        assert state.dataset_items[0]["request_index"] == 1
        assert len(state.charges) == 1, state.charges
        assert state.output == {
            "listings_requested": 2,
            "listings_completed": 2,
            "listings_saved": 1,
            "listings_failed": 1,
            "status_message": None,
            "failures": [
                {
                    "ad_id": REMOVED_ID,
                    "error": "Scrappa API error (404): removed",
                    "outcome": "failed",
                }
            ],
        }, state.output

        paths = [request[1].split("?", 1)[0] for request in state.requests]
        assert paths.index("/v2/datasets/dataset-123/items") < paths.index(
            "/v2/actor-runs/run-123/charge"
        )
        assert paths.index("/v2/actor-runs/run-123/charge") < paths.index(
            "/v2/key-value-stores/store-123/records/OUTPUT"
        )
        print(
            "Local image smoke passed: container input, Scrappa request, result charge, "
            "dataset row, and OUTPUT record verified."
        )
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
