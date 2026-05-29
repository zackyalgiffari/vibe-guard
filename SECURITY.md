# Security Policy

## Supported versions

`vibe-guard` is pre-1.0. Security fixes are applied to the latest released version on the
`main` branch. Please make sure you are on the most recent version before reporting.

## Reporting a vulnerability

Please **do not** open a public issue for security vulnerabilities.

Instead, report privately using GitHub's
[private vulnerability reporting](https://github.com/zackyalgiffari/vibe-guard/security/advisories/new)
("Report a vulnerability" under the repository's **Security** tab). If that is unavailable,
contact the maintainer directly.

When reporting, please include:

- A description of the issue and its impact.
- Steps to reproduce (a minimal diff or input file is ideal — redact any real secrets).
- The version / commit you tested against.

You can expect an initial acknowledgement within a few days. Once a fix is available we
will coordinate disclosure and credit you, unless you prefer to remain anonymous.

## Scope notes

`vibe-guard` runs fully locally and makes no remote calls except to a user-configured
local Ollama endpoint. The safety guard's secret/sensitive-file detection is a
best-effort heuristic and **not** a guarantee — treat it as a helpful warning layer, not
a substitute for proper secret scanning and review.
