# Feature: docs-sweep (documentation for sonda)

Status: in progress
Branch: docs/documentation-sweep (from master e003c09)

## Goal

Documentation set for the sonda CLI following cognitive-doc-design patterns
(answer first, progressive disclosure, tables over prose): contract v1 field
spec, architecture guide with a new-probe checklist, Keep-a-Changelog
CHANGELOG, and an expanded README. English (family convention).

## Decisions (user-confirmed)

- Scope: contract spec + architecture + CHANGELOG + README expansion.
- Delivery: PR docs/documentation-sweep with issue-first flow (type:docs).
- Idioma: inglés.

## Tasks

- [x] T1: docs/contract.md — field-by-field v1 spec with real example — this commit
- [x] T2: docs/architecture.md — probes, conventions, new-probe checklist — this commit
- [x] T3: CHANGELOG.md (Keep a Changelog, 0.1.0) — this commit
- [x] T4: README expansion (probes table, jq quickstart, troubleshooting) — this commit
- [x] T5: PR with issue-first flow + type:docs label — opened after this commit

## Evidence log

- Single work-unit commit: docs sweep, 4 files. 48/48 tests still green;
  all three README jq recipes verified live against a real scan before
  committing (recipes documented exactly as verified).
