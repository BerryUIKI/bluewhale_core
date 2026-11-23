# Architecture decision records

An architecture decision record (ADR) captures one significant decision: the context, the decision, its consequences, and the alternatives considered. ADRs are immutable once accepted; a later ADR supersedes an earlier one instead of editing it.

The R1-R6 review decisions live in their review documents under `docs/`. ADRs are for decisions made after that review phase.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| — | No ADRs recorded yet | — |

## Creating an ADR

1. Copy `0000-template.md` to `NNNN-short-title.md`, using the next number.
2. Set the status to `proposed` and open a pull request.
3. After approval, change the status to `accepted` and add it to the index table.
4. If a decision changes, add a new ADR and mark the old one `superseded by ADR-NNNN`.
