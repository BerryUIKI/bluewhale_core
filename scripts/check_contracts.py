#!/usr/bin/env python3
"""Validate the language-neutral contracts in `contracts/`.

Checks, in order:

1. Every `*.schema.json` parses and is a valid JSON Schema 2020-12 document.
2. Every fixture manifest lists existing files, its SHA-256 values match, each
   listed document exists in its schema, and every fixture file is listed.
3. Each fixture validates (or is rejected) as the manifest says.
4. The OpenAPI document parses, every external schema reference resolves, and
   the bundled document is a valid OpenAPI 3.1 description. Every object-typed
   schema referenced by the API has at least one valid fixture case.

Requires: `jsonschema`, `PyYAML`, and `openapi-spec-validator`.
Run with `python -I scripts/check_contracts.py` from anywhere.
"""

from __future__ import annotations

import copy
import hashlib
import json
import sys
from pathlib import Path

import yaml
from jsonschema import Draft202012Validator
from openapi_spec_validator import validate as validate_openapi

REPO_ROOT = Path(__file__).resolve().parent.parent
CONTRACTS = REPO_ROOT / "contracts"
EXTERNAL_REF_PREFIX = "./sync.schema.json#/$defs/"
INTERNAL_REF_PREFIX = "#/$defs/"
COMPONENT_REF_PREFIX = "#/components/schemas/"

failures: list[str] = []


def fail(message: str) -> None:
    failures.append(message)
    print(f"FAIL: {message}")


def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail(f"{path.relative_to(REPO_ROOT)}: {error}")
        return None


def check_schemas() -> dict[Path, dict]:
    schemas: dict[Path, dict] = {}
    for path in sorted(CONTRACTS.rglob("*.schema.json")):
        schema = load_json(path)
        if schema is None:
            continue
        try:
            Draft202012Validator.check_schema(schema)
        except Exception as error:  # noqa: BLE001 - report the validator's message
            fail(f"{path.relative_to(REPO_ROOT)}: invalid JSON Schema: {error}")
            continue
        schemas[path] = schema
    return schemas


def validator_for(schema: dict, document: str) -> Draft202012Validator:
    wrapper = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": f"#/$defs/{document}",
        "$defs": schema["$defs"],
    }
    return Draft202012Validator(wrapper)


def check_manifest(manifest_path: Path, schemas: dict[Path, dict]) -> None:
    manifest = load_json(manifest_path)
    if manifest is None:
        return
    directory = manifest_path.parent
    relative = manifest_path.relative_to(REPO_ROOT)

    if manifest.get("manifest_version") != 1:
        fail(f"{relative}: unsupported manifest_version")
    schema_name = manifest.get("schema")
    schema_path = directory.parent / schema_name if isinstance(schema_name, str) else None
    schema = schemas.get(schema_path) if schema_path is not None else None
    if schema is None:
        fail(f"{relative}: schema {schema_name!r} is missing or invalid")
        return

    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        fail(f"{relative}: cases must be a non-empty array")
        return

    listed: set[str] = set()
    documents: set[str] = set()
    for case in cases:
        if not isinstance(case, dict):
            fail(f"{relative}: a case is not an object")
            continue
        name = case.get("file")
        document = case.get("document")
        expected_valid = case.get("valid")
        expected_hash = case.get("sha256")
        if not all(isinstance(value, str) for value in (name, document, expected_hash)):
            fail(f"{relative}: a case has a non-string field")
            continue
        if not isinstance(expected_valid, bool):
            fail(f"{relative}: {name} has a non-boolean valid flag")
            continue
        if name in listed:
            fail(f"{relative}: duplicate case {name}")
            continue
        listed.add(name)
        documents.add(document)

        fixture_path = directory / name
        try:
            payload = fixture_path.read_bytes()
        except OSError as error:
            fail(f"{relative}: {name}: {error}")
            continue
        digest = hashlib.sha256(payload).hexdigest()
        if digest != expected_hash:
            fail(f"{relative}: {name}: sha256 {digest} does not match the manifest")

        if document not in schema.get("$defs", {}):
            fail(f"{relative}: {name}: unknown document {document}")
            continue
        try:
            instance = json.loads(payload)
        except json.JSONDecodeError as error:
            fail(f"{relative}: {name}: {error}")
            continue
        errors = list(validator_for(schema, document).iter_errors(instance))
        if errors and expected_valid:
            fail(f"{relative}: {name}: expected valid, got {errors[0].message}")
        if not errors and not expected_valid:
            fail(f"{relative}: {name}: expected rejection, but the document is valid")

    on_disk = {
        str(path.relative_to(directory)).replace("\\", "/")
        for path in directory.rglob("*.json")
        if path != manifest_path
    }
    for missing in sorted(on_disk - listed):
        fail(f"{relative}: {missing} is not listed in the manifest")

    for document in sorted(documents):
        if document not in schema.get("$defs", {}):
            fail(f"{relative}: manifest document {document} is missing from the schema")


def rewrite_internal(node) -> None:
    if isinstance(node, dict):
        reference = node.get("$ref")
        if isinstance(reference, str) and reference.startswith(INTERNAL_REF_PREFIX):
            node["$ref"] = COMPONENT_REF_PREFIX + reference[len(INTERNAL_REF_PREFIX):]
            return
        for value in node.values():
            rewrite_internal(value)
    elif isinstance(node, list):
        for item in node:
            rewrite_internal(item)


def rewrite_external(node) -> None:
    if isinstance(node, dict):
        reference = node.get("$ref")
        if isinstance(reference, str) and reference.startswith(EXTERNAL_REF_PREFIX):
            node["$ref"] = COMPONENT_REF_PREFIX + reference[len(EXTERNAL_REF_PREFIX):]
            return
        for value in node.values():
            rewrite_external(value)
    elif isinstance(node, list):
        for item in node:
            rewrite_external(item)


def check_openapi(schemas: dict[Path, dict]) -> None:
    for path in sorted(CONTRACTS.rglob("openapi.yaml")):
        try:
            spec = yaml.safe_load(path.read_text(encoding="utf-8"))
        except (OSError, yaml.YAMLError) as error:
            fail(f"{path.relative_to(REPO_ROOT)}: {error}")
            continue
        relative = path.relative_to(REPO_ROOT)
        if not str(spec.get("openapi", "")).startswith("3.1."):
            fail(f"{relative}: expected OpenAPI 3.1")

        schema_name = str(spec.get("info", {}).get("x-schema", "sync.schema.json"))
        schema_path = path.parent / schema_name
        schema = schemas.get(schema_path)
        if schema is None:
            fail(f"{relative}: schema {schema_name!r} is missing or invalid")
            continue

        referenced: set[str] = set()

        def collect(node) -> None:
            if isinstance(node, dict):
                reference = node.get("$ref")
                if isinstance(reference, str) and reference.startswith(EXTERNAL_REF_PREFIX):
                    referenced.add(reference[len(EXTERNAL_REF_PREFIX):])
                for value in node.values():
                    collect(value)
            elif isinstance(node, list):
                for item in node:
                    collect(item)

        collect(spec)
        defs = schema.get("$defs", {})
        for document in sorted(referenced):
            if document not in defs:
                fail(f"{relative}: unresolved reference {EXTERNAL_REF_PREFIX}{document}")

        components = spec.setdefault("components", {})
        component_schemas = components.setdefault("schemas", {})
        for name, definition in defs.items():
            bundled = copy.deepcopy(definition)
            rewrite_internal(bundled)
            component_schemas[name] = bundled
        rewrite_external(spec)

        try:
            validate_openapi(spec)
        except Exception as error:  # noqa: BLE001 - report the validator's message
            fail(f"{relative}: invalid OpenAPI: {error}")
            continue

        manifest_path = None
        for candidate in sorted(path.parent.rglob("manifest.json")):
            manifest_path = candidate
            break
        if manifest_path is None:
            fail(f"{relative}: no fixture manifest beside the OpenAPI document")
            continue
        manifest = load_json(manifest_path)
        if manifest is None:
            continue
        documented = {
            case.get("document")
            for case in manifest.get("cases", [])
            if case.get("valid") is True
        }
        for document in sorted(referenced):
            definition = defs.get(document, {})
            if definition.get("type") == "object" and document not in documented:
                fail(f"{relative}: no valid fixture for {document}")


def main() -> int:
    schemas = check_schemas()
    manifests = sorted(CONTRACTS.rglob("manifest.json"))
    if not manifests:
        fail("no fixture manifest found")
    for manifest in manifests:
        check_manifest(manifest, schemas)
    check_openapi(schemas)

    if failures:
        print(f"{len(failures)} contract check(s) failed")
        return 1
    cases = 0
    for manifest in manifests:
        document = load_json(manifest)
        if document is not None:
            cases += len(document.get("cases", []))
    print(
        f"contracts ok: {len(schemas)} schema(s), {len(manifests)} manifest(s), {cases} fixture(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
