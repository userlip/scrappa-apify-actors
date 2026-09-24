#!/usr/bin/env python3
"""Build and run the production image against local Apify and Scrappa mocks."""

import json
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse


ACTOR_DIR = Path(__file__).resolve().parents[1]
IMAGE = "jameda-doctor-details-scraper:local-smoke"
DOCTOR_URL = "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin"


class MockState:
    def __init__(self):
        self.dataset_items = []
        self.charges = []
        self.output = None
        self.status = None
        self.scrappa_requests = []


def json_response(handler, status, value):
    body = json.dumps(value).encode()
    handler.send_response(status)
    handler.send_header("Content-Type", "application/json")
    handler.send_header("Content-Length", str(len(body)))
    handler.end_headers()
    handler.wfile.write(body)


def make_handler(state, is_scrappa):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            parsed = urlparse(self.path)
            if is_scrappa:
                state.scrappa_requests.append((self.path, self.headers.get("X-API-Key")))
                if parsed.path != "/api/jameda/doctor-details":
                    return json_response(self, 404, {"message": "unknown Scrappa path"})
                if parse_qs(parsed.query).get("doctor_url") != [DOCTOR_URL]:
                    return json_response(self, 400, {"message": "doctor_url was not normalized"})
                if self.headers.get("X-API-Key") != "smoke-scrappa-key":
                    return json_response(self, 401, {"message": "missing Scrappa key"})
                return json_response(self, 200, {
                    "success": True,
                    "meta": {"source": "scrappa", "scraped_at": "2026-06-20T00:00:00Z"},
                    "data": {"basic_info": {"name": "Smoke Doctor", "profile_url": DOCTOR_URL}},
                })

            if parsed.path == "/api/v2/key-value-stores/smoke-store/records/INPUT":
                return json_response(self, 200, {"doctorUrl": DOCTOR_URL})
            if parsed.path == "/api/v2/actor-runs/smoke-run":
                return json_response(self, 200, {"data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "doctor-profile-result": {"eventPriceUsd": 0.001},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        }},
                    },
                    "chargedEventCounts": {},
                    "options": {"maxTotalChargeUsd": 0.0011},
                }})
            return json_response(self, 404, {"message": "unknown Apify GET path"})

        def do_POST(self):
            body = self.read_body()
            if self.path == "/api/v2/datasets/smoke-dataset/items":
                state.dataset_items.append(body)
                return json_response(self, 201, {})
            if self.path == "/api/v2/actor-runs/smoke-run/charge":
                state.charges.append((body, self.headers.get("Idempotency-Key")))
                return json_response(self, 201, {})
            return json_response(self, 404, {"message": "unknown Apify POST path"})

        def do_PUT(self):
            body = self.read_body()
            if self.path == "/api/v2/key-value-stores/smoke-store/records/OUTPUT":
                state.output = body
                return json_response(self, 201, {})
            if self.path == "/api/v2/actor-runs/smoke-run":
                state.status = body
                return json_response(self, 200, {"data": {"id": "smoke-run"}})
            return json_response(self, 404, {"message": "unknown Apify PUT path"})

        def read_body(self):
            length = int(self.headers.get("Content-Length", "0"))
            return json.loads(self.rfile.read(length) or b"null")

        def log_message(self, _format, *_args):
            pass

    return Handler


def start_server(state, is_scrappa):
    server = ThreadingHTTPServer(
        ("0.0.0.0", 0), make_handler(state, is_scrappa)
    )
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, f"http://127.0.0.1:{server.server_port}"


def run_checked(command, **options):
    result = subprocess.run(command, text=True, capture_output=True, **options)
    if result.returncode:
        raise RuntimeError(
            f"Command failed ({result.returncode}): {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def main():
    state = MockState()
    apify_server, apify_base = start_server(state, is_scrappa=False)
    scrappa_server, scrappa_base = start_server(state, is_scrappa=True)
    try:
        print("Building local Jameda Rust actor image...")
        run_checked(
            ["docker", "build", "-f", ".actor/Dockerfile", "-t", IMAGE, "."],
            cwd=ACTOR_DIR,
        )
        environment = {
            "APIFY_API_PUBLIC_BASE_URL": f"{apify_base}/api",
            "APIFY_TOKEN": "smoke-apify-token",
            "ACTOR_RUN_ID": "smoke-run",
            "ACTOR_DEFAULT_KEY_VALUE_STORE_ID": "smoke-store",
            "ACTOR_DEFAULT_DATASET_ID": "smoke-dataset",
            "ACTOR_INPUT_KEY": "INPUT",
            "SCRAPPA_API_BASE_URL": f"{scrappa_base}/api",
            "SCRAPPA_API_KEY": "smoke-scrappa-key",
        }
        print("Running production image against local Apify and Scrappa mocks...")
        run_checked([
            "docker", "run", "--rm", "--network", "host",
            *[argument for key, value in environment.items() for argument in ("-e", f"{key}={value}")],
            IMAGE,
        ])

        assert len(state.scrappa_requests) == 1, state.scrappa_requests
        assert state.dataset_items == [{
            "success": True,
            "meta": {"source": "scrappa", "scraped_at": "2026-06-20T00:00:00Z"},
            "data": {"basic_info": {"name": "Smoke Doctor", "profile_url": DOCTOR_URL}},
            "requested_doctor_url": DOCTOR_URL,
            "doctor_url": DOCTOR_URL,
            "doctor_name": "Smoke Doctor",
            "title": None,
            "specialty": None,
            "description": None,
            "rating": None,
            "rating_number": None,
            "review_count": None,
            "review_count_number": None,
            "clinic_name": None,
            "phone": None,
            "website_url": None,
            "address": None,
            "city": None,
            "postal_code": None,
            "latitude": None,
            "longitude": None,
            "image_url": None,
            "services_count": None,
            "focus_areas_count": None,
            "conditions_count": None,
            "languages_count": None,
            "opening_hours": None,
            "services": None,
            "accepted_patients": None,
            "focus_areas": None,
            "conditions": None,
            "languages": None,
            "booking_ids": None,
            "request_doctor_url": DOCTOR_URL,
            "response_source": "scrappa",
            "scraped_at": "2026-06-20T00:00:00Z",
        }], state.dataset_items
        assert state.charges == [(
            {"eventName": "doctor-profile-result", "count": 1},
            "smoke-run-doctor-profile-result-1",
        )], state.charges
        assert state.output["doctors_requested"] == 1, state.output
        assert state.output["doctors_saved"] == 1, state.output
        assert state.output["doctors_failed"] == 0, state.output
        assert state.status is None, state.status
        print("Local image smoke passed: authenticated request, dataset item, PPE charge, and OUTPUT record verified.")
    finally:
        apify_server.shutdown()
        scrappa_server.shutdown()
        apify_server.server_close()
        scrappa_server.server_close()


if __name__ == "__main__":
    main()
