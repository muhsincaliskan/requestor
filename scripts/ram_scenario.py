"""Writes a Requestor data folder with open tabs and a response fixture."""
import json
import os
import sys

COLLECTION_ID = "ram-scenario"
BYTES_PER_MEGABYTE = 1024 * 1024
FIXTURE_NAME = "response.json"


def request_item(index, url):
    return {
        "id": f"request-{index}",
        "name": f"Request {index}",
        "request": {
            "method": "GET",
            "url": url,
            "header": [{"key": "Accept", "value": "application/json"}],
        },
    }


def fixture_text(megabytes):
    record = {"id": 0, "name": "Sample user", "email": "user@example.test", "tags": ["a", "b"]}
    size = len(json.dumps(record)) + 1
    count = megabytes * BYTES_PER_MEGABYTE // size
    return json.dumps([dict(record, id=number) for number in range(count)])


def main():
    work, tabs, megabytes, port = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    home = os.path.join(work, "home")
    fixture = os.path.join(work, "fixture")
    os.makedirs(os.path.join(home, "collections"))
    os.makedirs(fixture)
    url = f"http://127.0.0.1:{port}/{FIXTURE_NAME}"
    collection = {
        "info": {
            "_postman_id": COLLECTION_ID,
            "name": "RAM scenario",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json",
        },
        "item": [request_item(index, url) for index in range(tabs)],
    }
    session = {
        "tabs": [
            {"collection_id": COLLECTION_ID, "item_id": f"request-{index}", "name": f"Request {index}"}
            for index in range(tabs)
        ],
        "active_tab": 0,
    }
    with open(os.path.join(home, "collections", f"{COLLECTION_ID}.json"), "w") as handle:
        json.dump(collection, handle)
    with open(os.path.join(home, "session.json"), "w") as handle:
        json.dump(session, handle)
    with open(os.path.join(fixture, FIXTURE_NAME), "w") as handle:
        handle.write(fixture_text(megabytes))


main()
