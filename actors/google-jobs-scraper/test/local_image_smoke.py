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

POSITIVE_CAP_PRICING = {
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

NON_PPE_PRICING = {"data": {"pricingInfo": {"pricingModel": "FREE"}}}


def ppe_pricing(max_total_charge=None, include_max_total_charge=True):
    data = {
        "pricingInfo": {
            "pricingModel": "PAY_PER_EVENT",
            "pricingPerEvent": {
                "actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.01},
                    "apify-actor-start": {"eventPriceUsd": 0.005},
                }
            },
        },
        "chargedEventCounts": {"apify-actor-start": 1},
    }
    if include_max_total_charge:
        data["options"] = {"maxTotalChargeUsd": max_total_charge}
    else:
        data["options"] = {}
    return {"data": data}


RUN_PRICING = {
    "ppe-positive": POSITIVE_CAP_PRICING,
    "free": NON_PPE_PRICING,
    "ppe-missing-limit": ppe_pricing(include_max_total_charge=False),
    "ppe-null-limit": ppe_pricing(max_total_charge=None),
    "ppe-zero-limit": ppe_pricing(max_total_charge=0.0),
}


class SmokeServer(ThreadingHTTPServer):
    def __init__(self, address, handler):
        super().__init__(address, handler)
        self.calls = []
        self.dataset_items = []
        self.outputs = []
        self.state_lock = threading.Lock()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, _format, *_args):
        pass

    def do_GET(self):
        self._record_call()
        if self.path == "/v2/key-value-stores/test-store/records/INPUT":
            self._respond(200, INPUT)
            return
        if urlsplit(self.path).path.startswith("/v2/actor-runs/"):
            run_id = urlsplit(self.path).path.rsplit("/", 1)[-1]
            if run_id not in RUN_PRICING:
                self._respond(404, {"error": "unexpected actor run", "path": self.path})
                return
            self._respond(200, RUN_PRICING[run_id])
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
            self.server.outputs.append(output)
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


def run_smoke_case(image, server, port, run_id, expected_items, capped):
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
        "ACTOR_RUN_ID={}".format(run_id),
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
    with server.state_lock:
        previous_call_count = len(server.calls)
        previous_item_count = len(server.dataset_items)
        previous_output_count = len(server.outputs)

    result = subprocess.run(command, capture_output=True, text=True, timeout=60)
    with server.state_lock:
        calls = list(server.calls[previous_call_count:])
        dataset_items = list(server.dataset_items[previous_item_count:])
        outputs = list(server.outputs[previous_output_count:])

    if result.returncode != 0:
        raise RuntimeError(
            "Actor image exited {} for {}\nstdout:\n{}\nstderr:\n{}\nmock service calls:\n{}".format(
                result.returncode, run_id, result.stdout, result.stderr, json.dumps(calls, indent=2)
            )
        )

    google_calls = [call for call in calls if urlsplit(call["path"]).path == "/api/google/jobs"]
    assert len(google_calls) == 1, "expected one Google Jobs request for {}: {}".format(run_id, google_calls)
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
    assert all("/charges" not in call["path"].lower() for call in apify_calls), apify_calls
    assert dataset_items == expected_items, "{} dataset rows: {}".format(run_id, dataset_items)
    assert outputs == [GOOGLE_RESPONSE], "{} OUTPUT: {}".format(run_id, outputs)
    if capped:
        assert "Charge limit reached after saving 1/2" in result.stdout, result.stdout
    else:
        assert "Charge limit reached" not in result.stdout, result.stdout


def run_smoke(image):
    server = SmokeServer(("0.0.0.0", 0), Handler)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    port = server.server_address[1]
    try:
        run_smoke_case(
            image,
            server,
            port,
            "ppe-positive",
            [GOOGLE_RESPONSE["jobs"][0]],
            capped=True,
        )
        all_jobs = GOOGLE_RESPONSE["jobs"]
        for run_id in ["free", "ppe-missing-limit", "ppe-null-limit", "ppe-zero-limit"]:
            run_smoke_case(image, server, port, run_id, all_jobs, capped=False)
    finally:
        server.shutdown()
        server.server_close()
        server_thread.join(timeout=5)
    print("Local image smoke passed: positive PPE cap, FREE run, missing/null/zero unlimited caps, dataset rows, and OUTPUT verified.")


if __name__ == "__main__":
    image = sys.argv[1] if len(sys.argv) > 1 else "google-jobs-scraper:local"
    run_smoke(image)
