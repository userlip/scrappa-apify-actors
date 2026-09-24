#!/usr/bin/env python3
"""Run the built actor image against local Apify and Scrappa API doubles."""

import argparse
import json
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit


INPUT = {
    "items": [
        {"text": "Good morning", "source": "en", "target": "de"},
        {"text": "Bad input", "source": "en", "target": "es"},
    ]
}


class SmokeState:
    def __init__(self, transient_charge_failures=0, reject_charge=False):
        self.dataset_items = []
        self.charges = []
        self.charge_attempts = []
        self.accepted_charge_keys = {}
        self.transient_charge_failures = transient_charge_failures
        self.reject_charge = reject_charge
        self.output = None
        self.status_messages = []
        self.request_order = []
        self.failures = []


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        path = urlsplit(self.path).path
        if path == "/api/v2/actor-runs/smoke-run":
            self._require_apify_auth()
            self._write_json(
                200,
                {
                    "data": {
                        "pricingInfo": {
                            "pricingModel": "PAY_PER_EVENT",
                            "pricingPerEvent": {
                                "actorChargeEvents": {
                                    "translation-result": {
                                        "eventTieredPricingUsd": {
                                            "FREE": {"tieredEventPriceUsd": 0.0006},
                                            "GOLD": {"tieredEventPriceUsd": 0.0002},
                                        }
                                    },
                                    "apify-default-dataset-item": {
                                        "eventTieredPricingUsd": {
                                            "FREE": {"tieredEventPriceUsd": 0.0004},
                                            "GOLD": {"tieredEventPriceUsd": 0.0001},
                                        }
                                    },
                                }
                            },
                        },
                        "options": {"maxTotalChargeUsd": 0.0005},
                        "chargedEventCounts": {},
                    }
                },
            )
            return

        if path == "/api/v2/key-value-stores/smoke-store/records/INPUT":
            self._require_apify_auth()
            self._write_json(200, INPUT)
            return

        if path == "/api/google-translate":
            self._require_scrappa_auth()
            query = parse_qs(urlsplit(self.path).query)
            self._require(set(query) == {"text", "source", "target"}, "unexpected Scrappa query parameters")
            if query["text"] == ["Bad input"]:
                self._write_json(400, {"message": "Invalid target language."})
                return
            self._write_json(200, {"translated_text": "Guten Morgen"})
            return

        self._write_json(404, {"message": f"Unexpected GET path: {path}"})

    def do_POST(self):
        path = urlsplit(self.path).path
        body = self._read_json()
        if path == "/api/v2/datasets/smoke-dataset/items":
            self._require_apify_auth()
            self.server.state.dataset_items.append(body)
            self.server.state.request_order.append("dataset")
            self._write_json(201, {})
            return

        if path == "/api/v2/actor-runs/smoke-run/charge":
            self._require_apify_auth()
            idempotency_key = self.headers.get("idempotency-key")
            self._require(bool(idempotency_key), "missing charge idempotency key")
            state = self.server.state
            state.charge_attempts.append((body, idempotency_key))
            if idempotency_key in state.accepted_charge_keys:
                self._require(
                    state.accepted_charge_keys[idempotency_key] == body,
                    "charge retry changed the idempotent request body",
                )
                state.request_order.append("charge-replay")
                self._write_json(201, {})
                return
            if state.reject_charge:
                state.request_order.append("charge-rejected")
                self._write_json(402, {"message": "Charge rejected by smoke fixture."})
                return

            state.accepted_charge_keys[idempotency_key] = body
            state.charges.append(body)
            state.request_order.append("charge")
            if state.transient_charge_failures > 0:
                state.transient_charge_failures -= 1
                self._write_json(503, {"message": "Temporary charge service error."})
                return
            self._write_json(201, {})
            return

        self._write_json(404, {"message": f"Unexpected POST path: {path}"})

    def do_PUT(self):
        path = urlsplit(self.path).path
        body = self._read_json()
        if path == "/api/v2/key-value-stores/smoke-store/records/OUTPUT":
            self._require_apify_auth()
            self.server.state.output = body
            self._write_json(200, {})
            return

        if path == "/api/v2/actor-runs/smoke-run":
            self._require_apify_auth()
            self.server.state.status_messages.append(body)
            self._write_json(200, {})
            return

        self._write_json(404, {"message": f"Unexpected PUT path: {path}"})

    def _read_json(self):
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length)
        return json.loads(raw) if raw else None

    def _write_json(self, status, payload):
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _require(self, condition, message):
        if not condition:
            self.server.state.failures.append(message)
            raise AssertionError(message)

    def _require_apify_auth(self):
        self._require(self.headers.get("Authorization") == "Bearer smoke-token", "bad Apify bearer token")

    def _require_scrappa_auth(self):
        self._require(self.headers.get("X-API-Key") == "smoke-key", "bad Scrappa API key")
        self._require(
            self.headers.get("User-Agent") == "thescrappa-google-translate-scraper/1.0",
            "bad Scrappa actor user agent",
        )

    def log_message(self, _format, *_args):
        return


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--image", default="google-translate-scraper:local")
    args = parser.parse_args()

    state = SmokeState(transient_charge_failures=1)
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.state = state
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    port = server.server_address[1]
    api_base = f"http://127.0.0.1:{port}/api"

    def run_image():
        return subprocess.run(
            [
                "docker",
                "run",
                "--rm",
                "--network",
                "host",
                "-e",
                f"APIFY_API_PUBLIC_BASE_URL={api_base}",
                "-e",
                "APIFY_TOKEN=smoke-token",
                "-e",
                "APIFY_USER_PRICING_TIER=GOLD",
                "-e",
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID=smoke-store",
                "-e",
                "ACTOR_DEFAULT_DATASET_ID=smoke-dataset",
                "-e",
                "ACTOR_RUN_ID=smoke-run",
                "-e",
                "ACTOR_INPUT_KEY=INPUT",
                "-e",
                "SCRAPPA_API_BASE_URL=" + api_base,
                "-e",
                "SCRAPPA_API_KEY=smoke-key",
                args.image,
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=60,
        )

    result = run_image()

    if result.returncode != 0:
        raise SystemExit(
            "Local image run failed.\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )
    if state.failures:
        raise SystemExit("Mock service assertion failures: " + "; ".join(state.failures))

    expected_items = [
        {
            "success": True,
            "index": 0,
            "text": "Good morning",
            "translated_text": "Guten Morgen",
            "source": "en",
            "target": "de",
            "error": None,
            "status_code": None,
        },
        {
            "success": False,
            "index": 1,
            "text": "Bad input",
            "translated_text": None,
            "source": "en",
            "target": "es",
            "error": "Scrappa API error (400): Invalid target language.",
            "status_code": 400,
        },
    ]
    expected_output = {
        "requested": 2,
        "succeeded": 1,
        "failed": 1,
        "saved": 2,
        "status_message": None,
    }
    if state.dataset_items != expected_items:
        raise SystemExit(f"Unexpected dataset output: {state.dataset_items!r}")
    if state.output != expected_output:
        raise SystemExit(f"Unexpected OUTPUT record: {state.output!r}")
    if state.charges != [{"eventName": "translation-result", "count": 1}]:
        raise SystemExit(f"Unexpected event charges: {state.charges!r}")
    if len(state.charge_attempts) != 2:
        raise SystemExit(f"Expected one idempotent charge retry: {state.charge_attempts!r}")
    if state.charge_attempts[0][1] != state.charge_attempts[1][1]:
        raise SystemExit(f"Charge retry changed idempotency key: {state.charge_attempts!r}")
    if state.request_order != ["charge", "charge-replay", "dataset", "dataset"]:
        raise SystemExit(f"Unexpected output/charge ordering: {state.request_order!r}")

    state.dataset_items.clear()
    state.charges.clear()
    state.charge_attempts.clear()
    state.accepted_charge_keys.clear()
    state.output = None
    state.status_messages.clear()
    state.request_order.clear()
    state.failures.clear()
    state.reject_charge = True

    result = run_image()
    if result.returncode == 0:
        raise SystemExit("Actor run succeeded despite a rejected translation charge.")
    if state.failures:
        raise SystemExit("Mock service assertion failures: " + "; ".join(state.failures))
    if state.dataset_items:
        raise SystemExit(f"Charge rejection still published successful dataset rows: {state.dataset_items!r}")
    if state.charges or state.request_order != ["charge-rejected"]:
        raise SystemExit(f"Unexpected rejected-charge side effects: {state.request_order!r}")

    server.shutdown()
    server.server_close()
    thread.join(timeout=5)
    print("Local Rust image charge-order and rejection smoke passed.")
    print("transient accepted charge retried once with the same idempotency key before dataset writes")
    print("rejected charge stopped the run before publishing dataset rows")
    print(result.stdout)


if __name__ == "__main__":
    main()
