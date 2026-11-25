#!/usr/bin/env python3
"""Enforce the P1.8 coverage gates from a cargo-llvm-cov JSON export.

Line coverage must be at least 80% for each public core crate. Lines are
counted once per source line: LLVM reports generic and duplicate build
instantiations separately, and summing the raw per-file summaries would count
the same source line several times. With ``--branch``, the critical modules
must have 100% branch coverage.
"""

from __future__ import annotations

import argparse
import json
import sys

LINE_THRESHOLD = 80.0

PACKAGE_PREFIXES = {
    "core-domain": "crates/core-domain/",
    "core-application": "crates/core-application/",
    "core-persistence": "crates/core-persistence/",
    "core-sync": "crates/core-sync/",
    "core-crypto": "crates/core-crypto/",
    "core-testkit": "crates/core-testkit/",
}

CRITICAL_FILES = {
    "crates/core-domain/src/money.rs": "money",
    "crates/core-domain/src/currency.rs": "currency-scale validation",
    "crates/core-domain/src/transaction.rs": "transfer construction",
    "crates/core-domain/src/balance.rs": "balance calculation",
    "crates/core-domain/src/calendar.rs": "month boundaries",
    "crates/core-domain/src/statistics.rs": "monthly statistics",
}


def normalized(path: str) -> str:
    return path.replace("\\", "/")


def totals(files: list[dict], key: str) -> tuple[int, int]:
    count = sum(file["summary"][key]["count"] or 0 for file in files)
    covered = sum(file["summary"][key]["covered"] or 0 for file in files)
    return count, covered


def unique_line_totals(files: list[dict]) -> tuple[int, int]:
    """Count each instrumented source line once across all instantiations."""
    count = 0
    covered = 0
    for file in files:
        segments = file.get("segments")
        if not segments:
            summary = file["summary"]["lines"]
            count += summary["count"] or 0
            covered += summary["covered"] or 0
            continue
        lines: dict[int, int] = {}
        for segment in segments:
            line, _, segment_count, has_count = segment[:4]
            if has_count:
                lines[line] = max(lines.get(line, 0), segment_count)
        count += len(lines)
        covered += sum(1 for value in lines.values() if value > 0)
    return count, covered


def percent(covered: int, count: int) -> float:
    return 100.0 if count == 0 else covered * 100.0 / count


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", help="cargo-llvm-cov JSON export")
    parser.add_argument(
        "--branch",
        action="store_true",
        help="also require 100%% branch coverage for the critical modules",
    )
    args = parser.parse_args()

    with open(args.report, encoding="utf-8") as handle:
        report = json.load(handle)
    files = [file for data in report["data"] for file in data["files"]]
    failures: list[str] = []

    for package, prefix in PACKAGE_PREFIXES.items():
        selected = [file for file in files if prefix in normalized(file["filename"])]
        count, covered = unique_line_totals(selected)
        if count == 0:
            print(f"{package}: no line data, skipped")
            continue
        value = percent(covered, count)
        print(f"{package}: {value:.1f}% lines ({covered}/{count})")
        if value < LINE_THRESHOLD:
            failures.append(
                f"{package} line coverage {value:.1f}% is below {LINE_THRESHOLD:.0f}%"
            )

    if args.branch:
        for suffix, label in CRITICAL_FILES.items():
            selected = [
                file
                for file in files
                if normalized(file["filename"]).endswith(suffix)
            ]
            if not selected:
                failures.append(f"{suffix}: no coverage data")
                continue
            count, covered = totals(selected, "branches")
            value = percent(covered, count)
            print(f"{suffix}: {value:.1f}% branches ({covered}/{count}) [{label}]")
            if count == 0:
                failures.append(f"{suffix}: branch coverage was not measured")
            elif covered < count:
                failures.append(f"{suffix} branch coverage {value:.1f}% is below 100%")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
