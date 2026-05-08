# Governance

## Decision Model

Farol is maintained by a single person today. Final decisions on direction, features, and merges are made by the maintainer listed in [MAINTAINERS.md](MAINTAINERS.md). This may change as the project grows and more contributors join.

## How Decisions Are Made

- **Small changes** (bug fixes, minor improvements): merged by any maintainer after CI passes.
- **Medium changes** (new features, refactors): require one approving review from a maintainer.
- **Large changes** (architecture, breaking changes, new subsystems): require an RFC (see `rfcs/` directory) and discussion period before implementation.

## RFCs

Significant changes go through the RFC process:

1. Open an issue with the `type:rfc` label.
2. Write a proposal in `rfcs/NNNN-title.md`.
3. Allow 7 days for discussion.
4. Maintainer makes final call (accept, reject, or request changes).

See [RFCS.md](RFCS.md) for the template and process details.

## Code of Conduct

Contributors are expected to be respectful and constructive. We follow the [Contributor Covenant](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
