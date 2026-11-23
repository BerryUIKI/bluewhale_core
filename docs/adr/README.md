# Architecture decision records

An architecture decision record (ADR) captures one significant decision: the context, the decision, its consequences, and the alternatives considered. ADRs are immutable once accepted; a later ADR supersedes an earlier one instead of editing it.

The R1-R6 review decisions live in their review documents under `docs/`. ADRs are for decisions made after that review phase.

## Index

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-workspace-layout-and-repository-ports.md) | Workspace layout and repository ports | Proposed |
| [0003](0003-cross-repo-consumption-and-versioning.md) | Cross-repository consumption and versioning | Proposed |
| [0004](0004-toolchain-ci-and-release.md) | Toolchain, CI, and release | Proposed |

## Creating an ADR

1. Copy `0000-template.md` to `NNNN-short-title.md`, using the next number.
2. Set the status to `proposed` and open a pull request.
3. After approval, change the status to `accepted` and add it to the index table.
4. If a decision changes, add a new ADR and mark the old one `superseded by ADR-NNNN`.
