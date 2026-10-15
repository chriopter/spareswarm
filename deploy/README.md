# Deployment

GitHub Actions tests and builds the image, scans it, then promotes the scanned
image to `ghcr.io/chriopter/spareswarm:production`. YoloDeploy pulls that tag
and restarts the fixed server-side Compose service. A brief interruption is expected.

GitHub Actions secrets:

- `DEPLOY_WEBHOOK_URL`: `https://yolodeploy.app/deploy`
- `CF_ACCESS_CLIENT_ID`: shared Cloudflare Access client ID
- `CF_ACCESS_CLIENT_SECRET`: shared Cloudflare Access client secret
- `DEPLOY_TOKEN`: SpareSwarm-specific YoloDeploy token

The request has no body and cannot select a repository or command. The server
owns its Compose configuration and application secrets. The public version is
checked after deployment. No deployment branch is required.

The server needs permission to pull the GHCR image if the package is private.
Back up SQLite before incompatible migrations. Rollback is manual and requires
an image and database schema that work together.
