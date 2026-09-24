#!/usr/bin/env python3
"""Build and run the production image against local Apify and Scrappa stubs."""

import json
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


ACTOR_DIR = Path(__file__).resolve().parents[1]
IMAGE = "immobilienscout24-price-insights-scraper:local-smoke"
API_TOKEN = "local-smoke-apify-token"
SCRAPPA_KEY = "local-smoke-scrappa-key"
RESULT_EVENT = "price-insight-result"


class LocalApis(BaseHTTPRequestHandler):
    captured = []
    input_value = None

    def log_message(self, _format, *_args):
        pass

    def send_json(self, status, value):
        payload = json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def record(self, body=None):
        length = int(self.headers.get("Content-Length", "0"))
        payload = self.rfile.read(length) if length else b""
        self.captured.append({
            "method": self.command,
            "path": self.path,
            "headers": {name.lower(): value for name, value in self.headers.items()},
            "body": body if body is not None else (json.loads(payload) if payload else None),
        })

    def do_GET(self):
        self.record()
        if self.path == "/v2/key-value-stores/smoke-store/records/INPUT":
            return self.send_json(200, self.input_value)
        if self.path == "/v2/actor-runs/smoke-run":
            return self.send_json(200, {
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {
                            "actorChargeEvents": {
                                RESULT_EVENT: {"eventPriceUsd": 0.0005},
                            },
                        },
                    },
                    "chargedEventCounts": {},
                    "options": {"maxTotalChargeUsd": 0},
                },
            })
        if self.path.startswith("/api/immobilienscout24/price-insights?"):
            location = parse_qs(urlsplit(self.path).query).get("location", [""])[0]
            return self.send_json(200, {
                "success": True,
                "location": location,
                "geocode": "1276003001",
                "currency": "EUR",
                "prices": {
                    "apartment_rent_per_m2": 12.72,
                    "apartment_buy_per_m2": 4189.04,
                    "house_rent_per_m2": 16.51,
                    "house_buy_per_m2": 4394.87,
                },
            })
        return self.send_json(404, {"message": "unexpected local smoke request"})

    def do_POST(self):
        self.record()
        if self.path in (
            "/v2/datasets/smoke-dataset/items",
            "/v2/actor-runs/smoke-run/charge",
        ):
            return self.send_json(201 if "/items" in self.path else 200, {})
        return self.send_json(404, {"message": "unexpected local smoke request"})

    def do_PUT(self):
        self.record()
        if self.path == "/v2/actor-runs/smoke-run":
            return self.send_json(200, {})
        return self.send_json(404, {"message": "unexpected local smoke request"})


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def main():
    schema = json.loads((ACTOR_DIR / ".actor/input_schema.json").read_text())
    LocalApis.input_value = {
        name: definition["prefill"]
        for name, definition in schema["properties"].items()
        if "prefill" in definition
    }
    require(LocalApis.input_value.get("locations") == ["Berlin"], "schema prefill changed")

    subprocess.run(
        ["docker", "build", "-f", ".actor/Dockerfile", "-t", IMAGE, "."],
        cwd=ACTOR_DIR,
        check=True,
    )

    server = ThreadingHTTPServer(("127.0.0.1", 0), LocalApis)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    api_base = f"http://127.0.0.1:{server.server_port}"
    try:
        run = subprocess.run(
            [
                "docker", "run", "--rm", "--network", "host",
                "-e", f"APIFY_API_PUBLIC_BASE_URL={api_base}",
                "-e", "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=smoke-store",
                "-e", "ACTOR_DEFAULT_DATASET_ID=smoke-dataset",
                "-e", "ACTOR_RUN_ID=smoke-run",
                "-e", "ACTOR_INPUT_KEY=INPUT",
                "-e", f"APIFY_TOKEN={API_TOKEN}",
                "-e", f"SCRAPPA_API_KEY={SCRAPPA_KEY}",
                "-e", f"SCRAPPA_API_BASE_URL={api_base}/api",
                IMAGE,
            ],
            text=True,
            capture_output=True,
            check=False,
        )
    finally:
        server.shutdown()
        server.server_close()
        server_thread.join(timeout=2)

    if run.returncode != 0:
        raise RuntimeError(f"image run failed ({run.returncode})\n{run.stdout}\n{run.stderr}")

    dataset_writes = [
        request for request in LocalApis.captured
        if request["method"] == "POST" and request["path"] == "/v2/datasets/smoke-dataset/items"
    ]
    charge_calls = [
        request for request in LocalApis.captured
        if request["method"] == "POST" and request["path"] == "/v2/actor-runs/smoke-run/charge"
    ]
    status_updates = [
        request for request in LocalApis.captured
        if request["method"] == "PUT" and request["path"] == "/v2/actor-runs/smoke-run"
    ]
    scrappa_calls = [
        request for request in LocalApis.captured
        if request["path"].startswith("/api/immobilienscout24/price-insights?")
    ]

    require(len(dataset_writes) == 1, "expected exactly one dataset result")
    require(dataset_writes[0]["body"][0]["request_location"] == "Berlin", "dataset row did not match prefill")
    require(dataset_writes[0]["headers"].get("authorization") == f"Bearer {API_TOKEN}", "dataset auth missing")
    require(len(charge_calls) == 1, "expected exactly one PPE charge")
    require(charge_calls[0]["body"] == {"eventName": RESULT_EVENT, "count": 1}, "PPE event mismatch")
    require(charge_calls[0]["headers"].get("idempotency-key") == "smoke-run-price-insight-result-0", "charge idempotency key missing")
    require(
        LocalApis.captured.index(dataset_writes[0]) < LocalApis.captured.index(charge_calls[0]),
        "dataset result was not written before its PPE charge",
    )
    require(len(status_updates) == 1, "terminal run status was not written")
    require(status_updates[0]["body"]["statusMessage"] == "Saved 1 of 1 requested location snapshot(s); 0 failed.", "unexpected terminal status")
    require(len(scrappa_calls) == 1, "expected one Scrappa request")
    require(scrappa_calls[0]["headers"].get("x-api-key") == SCRAPPA_KEY, "Scrappa auth missing")
    require("authorization" not in scrappa_calls[0]["headers"], "Apify token leaked to Scrappa")
    require(not any("/records/OUTPUT" in request["path"] for request in LocalApis.captured), "unexpected OUTPUT key write")

    print(run.stdout, end="")
    print("Local image smoke passed: prefill, Scrappa request, dataset row, PPE charge, and terminal status.")


if __name__ == "__main__":
    main()
