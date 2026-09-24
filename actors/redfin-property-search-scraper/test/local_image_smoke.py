#!/usr/bin/env python3
"""Run the built actor image against local Apify and Scrappa HTTP fixtures."""

import json
import os
import subprocess
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from urllib.parse import parse_qs, urlsplit


ACTOR_DIR = Path(__file__).resolve().parents[1]
IMAGE = os.environ.get("REDFIN_ACTOR_IMAGE", "redfin-property-search-scraper:local")
APIFY_TOKEN = "smoke-apify-token"
SCRAPPA_KEY = "smoke-scrappa-key"


class FixtureServer(ThreadingHTTPServer):
    def __init__(self, address, handler):
        super().__init__(address, handler)
        self.dataset_items = []
        self.charges = []
        self.scrappa_requests = []
        self.errors = []


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, status, value=None):
        body = b"" if value is None else json.dumps(value).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if body:
            self.wfile.write(body)

    def do_GET(self):
        parsed = urlsplit(self.path)
        if parsed.path == "/apify/v2/actor-runs/smoke-run":
            if self.headers.get("Authorization") != f"Bearer {APIFY_TOKEN}":
                self.server.errors.append("run request missing Apify bearer auth")
            self.respond(200, {
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "property-result": {"eventTieredPricingUsd": {
                                "FREE": {"tieredEventPriceUsd": 0.0003},
                                "BRONZE": {"tieredEventPriceUsd": 0.00025},
                                "SILVER": {"tieredEventPriceUsd": 0.00022},
                                "GOLD": {"tieredEventPriceUsd": 0.0002},
                            }},
                            "apify-default-dataset-item": {"eventTieredPricingUsd": {
                                "FREE": {"tieredEventPriceUsd": 0.0},
                                "BRONZE": {"tieredEventPriceUsd": 0.0},
                                "SILVER": {"tieredEventPriceUsd": 0.0},
                                "GOLD": {"tieredEventPriceUsd": 0.0},
                            }},
                        }},
                    },
                    "chargedEventCounts": {},
                    "options": {"maxTotalChargeUsd": 1.0},
                },
            })
            return
        if parsed.path == "/apify/v2/key-value-stores/smoke-store/records/INPUT":
            if self.headers.get("Authorization") != f"Bearer {APIFY_TOKEN}":
                self.server.errors.append("input request missing Apify bearer auth")
            self.respond(200, {
                "region_id": "16163",
                "region_type": "6",
                "market": "Seattle",
                "num_homes": 2,
                "property_types": "1,2,3",
                "status": "9",
                "page": 2,
            })
            return
        if parsed.path == "/scrappa/api/redfin/search":
            query = parse_qs(parsed.query)
            if self.headers.get("X-API-Key") != SCRAPPA_KEY:
                self.server.errors.append("Scrappa request missing API key")
            if query.get("region_id") != ["16163"] or query.get("market") != ["seattle"]:
                self.server.errors.append(f"unexpected Scrappa query: {query}")
            if query.get("property_types") != ["1,2,3"] or query.get("page") != ["2"]:
                self.server.errors.append(f"input filters did not reach Scrappa: {query}")
            self.server.scrappa_requests.append(query)
            self.respond(200, {"data": {
                "count": 2,
                "properties": [
                    {"property_id": "1001", "listing_id": "2001", "address": "100 Test Ave", "zip": 98101,
                     "price": "850000", "beds": "3", "baths": "2.5", "property_type": "1",
                     "status": "Active", "mls_number": 12345, "future_field": "preserved"},
                    {"property_id": "1002", "listing_id": "2002", "address": "200 Test Ave", "price": 950000,
                     "property_type": 2, "status": "Pending"},
                ],
            }})
            return
        self.server.errors.append(f"unexpected GET {self.path}")
        self.respond(404, {"message": "not found"})

    def do_POST(self):
        parsed = urlsplit(self.path)
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        try:
            value = json.loads(body) if body else None
        except json.JSONDecodeError as error:
            self.server.errors.append(f"invalid request JSON: {error}")
            self.respond(400, {"message": "invalid JSON"})
            return
        if self.headers.get("Authorization") != f"Bearer {APIFY_TOKEN}":
            self.server.errors.append("POST request missing Apify bearer auth")
        if parsed.path == "/apify/v2/datasets/smoke-dataset/items":
            if isinstance(value, list):
                self.server.dataset_items.extend(value)
            else:
                self.server.dataset_items.append(value)
            self.respond(201)
            return
        if parsed.path == "/apify/v2/actor-runs/smoke-run/charge":
            if value != {"eventName": "property-result", "count": 1}:
                self.server.errors.append(f"unexpected charge payload: {value}")
            if not self.headers.get("Idempotency-Key"):
                self.server.errors.append("charge request missing idempotency key")
            self.server.charges.append((self.headers.get("Idempotency-Key"), value))
            self.respond(201, {})
            return
        self.server.errors.append(f"unexpected POST {self.path}")
        self.respond(404, {"message": "not found"})


def check_schema_prefill():
    schema = json.loads((ACTOR_DIR / ".actor/input_schema.json").read_text())
    fields = schema["properties"]
    assert fields["region_id"]["default"] == 16163
    assert fields["region_type"]["default"] == "6"
    assert fields["market"]["default"] == "seattle"
    assert fields["property_types"]["prefill"] == "1,2,3"
    assert fields["status"]["default"] == "9"
    assert fields["num_homes"]["default"] == 50
    assert fields["page"]["default"] == 1
    assert fields["searches"]["maxItems"] == 25


def main():
    check_schema_prefill()
    server = FixtureServer(("0.0.0.0", 0), Handler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    port = server.server_address[1]
    command = [
        "docker", "run", "--rm", "--network", "host",
        "-e", f"APIFY_API_PUBLIC_BASE_URL=http://127.0.0.1:{port}/apify",
        "-e", "APIFY_TOKEN=" + APIFY_TOKEN,
        "-e", "ACTOR_RUN_ID=smoke-run",
        "-e", "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=smoke-store",
        "-e", "ACTOR_DEFAULT_DATASET_ID=smoke-dataset",
        "-e", "SCRAPPA_API_BASE_URL=http://127.0.0.1:{}/scrappa/api".format(port),
        "-e", "SCRAPPA_API_KEY=" + SCRAPPA_KEY,
        IMAGE,
    ]
    try:
        run = subprocess.run(command, capture_output=True, text=True, timeout=240)
        if run.returncode != 0:
            raise RuntimeError(f"actor image exited {run.returncode}:\n{run.stdout}\n{run.stderr}")
        assert not server.errors, "\n".join(server.errors)
        assert len(server.scrappa_requests) == 1, server.scrappa_requests
        assert len(server.dataset_items) == 2, server.dataset_items
        assert len(server.charges) == 2, server.charges
        assert len({key for key, _ in server.charges}) == 2, server.charges
        first, second = server.dataset_items
        assert first["property_id"] == 1001
        assert first["price"] == 850000
        assert first["zip"] == "98101"
        assert first["property_type_label"] == "House"
        assert first["request_search_index"] == 0
        assert first["request_page"] == 2
        assert first["future_field"] == "preserved"
        assert second["property_type_label"] == "Townhouse"
        print("Local Rust actor image smoke passed: Apify INPUT, Scrappa auth/filters, normalized dataset rows, and PPE charges.")
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
