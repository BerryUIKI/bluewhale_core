# ADR-0001: Record architecture decisions

- Status: accepted
- Date: 2026-11-23
- Deciders: Bluewhale maintainers

## Context

The R1-R6 review documents record the initial product and technical decisions. Decisions made after that review need a lightweight, durable record so contributors can understand why the code looks the way it does and so an accepted decision is not silently rewritten.

## Decision

Use architecture decision records in `docs/adr/`, numbered sequentially, following the template in this directory. An accepted ADR is immutable; a later ADR supersedes it and the old record is marked accordingly. The index in `README.md` lists every ADR and its status.

## Consequences

- Every significant architecture, data-model, protocol, crypto, or tooling decision gets a short written record.
- Reviewers can reject a change by pointing at the ADR it contradicts.
- The review documents remain the source for R1-R6; ADRs cover what comes after.

## Alternatives considered

| Alternative | Why it was not chosen |
| --- | --- |
| Keep decisions only in pull requests | Pull requests are hard to discover later and are not indexed. |
| Extend the R1-R6 documents indefinitely | Those documents are review snapshots; editing them erodes the approved baseline. |

## References

- [Documentation index](../README.md)
- [ADR template](0000-template.md)
