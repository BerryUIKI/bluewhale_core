# Bluewhale documentation

This directory contains the product, architecture, domain, sync, client, quality, and delivery documents for the Bluewhale project line. Documents are written in English.

## Index

| Area | Document | Status |
| --- | --- | --- |
| Product | [PRODUCT_SPEC.md](product/PRODUCT_SPEC.md) | R1 approved |
| Architecture | [ARCHITECTURE.md](architecture/ARCHITECTURE.md) | R2 approved |
| Architecture | [REPOSITORY_LAYOUT.md](architecture/REPOSITORY_LAYOUT.md) | Draft for owner review |
| Architecture | [CLIENT_STACK_DECISION.md](architecture/CLIENT_STACK_DECISION.md) | Supporting decision record |
| Domain | [DATA_MODEL.md](domain/DATA_MODEL.md) | R3 approved |
| Sync and server | [SYNC_AND_SERVER_DESIGN.md](sync/SYNC_AND_SERVER_DESIGN.md) | R4 approved |
| Clients | [CLIENT_DESIGN.md](clients/CLIENT_DESIGN.md) | R5 approved, conditional on Phase 0 evidence |
| Clients | [SPIKE_RESULTS.md](clients/SPIKE_RESULTS.md) | Complete |
| Quality | [QUALITY_AND_DELIVERY.md](quality/QUALITY_AND_DELIVERY.md) | R6 approved |
| Quality | [COVERAGE.md](quality/COVERAGE.md) | Active |
| Plan | [DEVELOPMENT_PLAN.md](plan/DEVELOPMENT_PLAN.md) | Active |
| Plan | [P1.0_APPLE_REUSE_ASSESSMENT.md](plan/P1.0_APPLE_REUSE_ASSESSMENT.md) | Approved |
| Development | [setup.md](development/setup.md) | Active |
| Development | [build-test.md](development/build-test.md) | Active |
| Development | [dependency-policy.md](development/dependency-policy.md) | Active |
| Development | [licensing.md](development/licensing.md) | Active |
| Reference | [versions.md](reference/versions.md) | Active |
| Reference | [export-format.md](reference/export-format.md) | Active |
| Reference | [apple-provenance.md](reference/apple-provenance.md) | Active |
| Decisions | [adr/README.md](adr/README.md) | Template and index |

## Conventions

- Use LF line endings and relative links between documents.
- A behavior, schema, or protocol change updates its design document and `CHANGELOG.md` in the same change.
- New architecture or product decisions use an ADR in `adr/` once the review phase is complete.
- The private Apple application is a behavioral reference only. Its code, assets, fixtures, and private documents must not be copied into this repository.
