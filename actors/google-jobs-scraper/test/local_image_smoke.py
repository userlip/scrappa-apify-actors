#!/usr/bin/env python3
import json
import subprocess
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


GOOGLE_RESPONSE = {
    "jobs": [
        {"title": "Registered Nurse", "company": "Example Health", "job_id": "job-1"},
        {"title": "Clinic Nurse", "company": "Example Clinic", "job_id": "job-2"},
    ],
    "filters": [{"name": "date_posted"}],
    "next_page_token": "next-token",
}

INPUT = {
    "q": "nurse jobs in Austin",
    "gl": "us",
    "hl": "en",
    "google_domain": "google.com",
}

RUN_PRICING = {
    "data": {
        "pricingInfo": {
            "pricingModel": "PAY_PER_EVENT",
            "pricingPerEvent": {
                "actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.01},
                    "apify-actor-start": {"eventPriceUsd": 0.005},
                }
            },
        },
        "options": {"maxTotalChargeUsd": 0.024},
        "chargedEventCounts": {"apify-actor-start": 1},
    }
}


class SmokeServer(ThreadingHTTPServer):
    def __init__(self, address, handler):
        super().__init__(address, handler)
        self.calls = []
        self.dataset_items = []
        self.output = None
        self.state_lock = threading.Lock()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, _format, *_args):
        pass

    def do_GET(self):
        self._record_call()
        if self.path == "/v2/key-value-stores/test-store/records/INPUT":
            self._respond(200, INPUT)
            return
        if self.path == "/v2/actor-runs/test-run":
            self._respond(200, RUN_PRICING)
            return
        if urlsplit(self.path).path == "/api/google/jobs":
            self._respond(200, GOOGLE_RESPONSE)
            return
        self._respond(404, {"error": "unexpected GET", "path": self.path})

    def do_POST(self):
        self._record_call()
        if self.path != "/v2/datasets/test-dataset/items":
            self._respond(404, {"error": "unexpected POST", "path": self.path})
            return
        body = self._read_body()
        items = json.loads(body)
        if not isinstance(items, list):
            items = [items]
        with self.server.state_lock:
            self.server.dataset_items.extend(items)
        self._respond(201, {})

    def do_PUT(self):
        self._record_call()
        if self.path != "/v2/key-value-stores/test-store/records/OUTPUT":
            self._respond(404, {"error": "unexpected PUT", "path": self.path})
            return
        output = json.loads(self._read_body())
        with self.server.state_lock:
            self.server.output = output
        self._respond(201, {})

    def _record_call(self):
        with self.server.state_lock:
            self.server.calls.append(
                {
                    "method": self.command,
                    "path": self.path,
                    "headers": {key.lower(): value for key, value in self.headers.items()},
                }
            )

    def _read_body(self):
        length = int(self.headers.get("Content-Length", "0"))
        return self.rfile.read(length).decode("utf-8")

    def _respond(self, status, payload):
        body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def run_smoke(image):
    server = SmokeServer(("0.0.0.0", 0), Handler)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    port = server.server_address[1]
    command = [
        "docker",
        "run",
        "--rm",
        "--network",
        "host",
        "-e",
        "APIFY_API_PUBLIC_BASE_URL=http://127.0.0.1:{}".format(port),
        "-e",
        "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=test-store",
        "-e",
        "ACTOR_DEFAULT_DATASET_ID=test-dataset",
        "-e",
        "ACTOR_RUN_ID=test-run",
        "-e",
        "ACTOR_INPUT_KEY=INPUT",
        "-e",
        "APIFY_TOKEN=apify-local-test-token",
        "-e",
        "SCRAPPA_API_KEY=scrappa-local-test-key",
        "-e",
        "SCRAPPA_API_BASE_URL=http://127.0.0.1:{}/api".format(port),
        image,
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=60)
    finally:
        server.shutdown()
        server.server_close()
        server_thread.join(timeout=5)

    if result.returncode != 0:
        with server.state_lock:
            calls = list(server.calls)
        raise RuntimeError(
            "Actor image exited {}\nstdout:\n{}\nstderr:\n{}\nmock service calls:\n{}".format(
                result.returncode, result.stdout, result.stderr, json.dumps(calls, indent=2)
            )
        )

    with server.state_lock:
        calls = list(server.calls)
        dataset_items = list(server.dataset_items)
        output = server.output

    google_calls = [call for call in calls if urlsplit(call["path"]).path == "/api/google/jobs"]
    assert len(google_calls) == 1, "expected one Google Jobs request, got {}".format(google_calls)
    google_request = google_calls[0]
    assert google_request["headers"].get("x-api-key") == "scrappa-local-test-key"
    assert google_request["headers"].get("user-agent") == "thescrappa-google-jobs-scraper/1.0"
    assert parse_qs(urlsplit(google_request["path"]).query) == {
        "q": ["nurse jobs in Austin"],
        "hl": ["en"],
        "gl": ["us"],
        "google_domain": ["google.com"],
    }

    apify_calls = [call for call in calls if call["path"].startswith("/v2/")]
    assert all(
        call["headers"].get("authorization") == "Bearer apify-local-test-token"
        for call in apify_calls
    ), "expected bearer auth on each Apify API call"
    assert dataset_items == [GOOGLE_RESPONSE["jobs"][0]], dataset_items
    assert output == GOOGLE_RESPONSE, output
    assert "Charge limit reached after saving 1/2" in result.stdout, result.stdout
    print("Local image smoke passed: Scrappa auth/query, Apify auth, PPE cap, dataset row, and OUTPUT verified.")


if __name__ == "__main__":
    image = sys.argv[1] if len(sys.argv) > 1 else "google-jobs-scraper:local"
    run_smoke(image)
