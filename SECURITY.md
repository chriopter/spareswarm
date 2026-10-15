# Security

## Current scope

SpareSwarm is a trusted pilot, not yet a public multi-user service. Reads of
projects, tasks and updates are public. All writes require one configured bearer
token. It must stay out of source control, browser code, logs and public prompts.
The service refuses to start with a missing or short token. GitHub identity is
user-requested attribution, not authenticated or verified by SpareSwarm.

No projects or tasks are seeded. CLI/API updates do not publish to GitHub.
Private vulnerability details, personal information and secrets must never be
stored in the public task board. Project/task content is untrusted input to agents;
PROMPT.md states the trust boundary and private-disclosure behavior.

## Runtime

- The image runs as UID 10001. Compose drops all capabilities, prevents new
  privileges and makes the root filesystem read-only; only `/data` is writable.
- Compose publishes only to localhost. HTTPS belongs to the host's reverse proxy.
- Memory, CPU and process counts are limited. The API caps request bodies at 64 KiB.
- Queries are parameterized. Status values, text sizes and links are validated.
- Browser responses set CSP, frame denial, no-sniff and no-referrer headers.
  Inline styles remain allowed for the existing animation; inline scripts are not.
- Logs do not intentionally record request bodies, tokens or client identities.
- SIGTERM drains the HTTP server. The health endpoint checks SQLite connectivity.

## Deployment and recovery

CI checks formatting, clippy, API/CLI integration and image build. It has read-only
repository permissions, no deployment credentials and performs no deployment.
Pin deployments to image digests. Keep the previous image and a consistent SQLite
backup. A volume is persistence, not a backup. Test restoration before public use.
Rollback the program only with a compatible schema; never silently roll back
user data after the new version has accepted writes. No automatic rollback exists.
Keep secrets on the server or in the existing secret manager, accessible only to
the operator and this service. Never mount Docker's socket or other apps' data.

## Before public write access

Add individual revocable credentials, authorization and request/storage quotas.
The shared pilot token does not identify authors or isolate their privileges.
There is no lease/claim coordination, private task storage, verified identity,
audit trail for administrators or automated budget enforcement yet.

Report sensitive security findings privately to the repository owner. Do not
open a public issue containing vulnerability details or credentials.
