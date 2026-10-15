"""Trigger the fixed YoloDeploy target through Cloudflare Access."""
import os
import urllib.error
import urllib.parse
import urllib.request


def required(name):
    value = os.environ.get(name, "")
    if not value:
        raise ValueError(f"Missing {name}")
    return value


def deployment_request():
    url = required("DEPLOY_WEBHOOK_URL")
    parsed = urllib.parse.urlsplit(url)
    if (parsed.scheme != "https" or not parsed.hostname or parsed.username
            or parsed.password or parsed.query or parsed.fragment or parsed.path != "/deploy"):
        raise ValueError("Webhook must use HTTPS and /deploy without credentials or query")
    return urllib.request.Request(url, data=b"", method="POST", headers={
        "User-Agent": "SpareSwarm-Deploy/1.0",
        "Authorization": "Bearer " + required("DEPLOY_TOKEN"),
        "CF-Access-Client-Id": required("CF_ACCESS_CLIENT_ID"),
        "CF-Access-Client-Secret": required("CF_ACCESS_CLIENT_SECRET"),
    })


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        return None


def main():
    with urllib.request.build_opener(NoRedirect).open(deployment_request(), timeout=100) as response:
        if response.status != 200:
            raise RuntimeError("Deployment was not accepted")
        response.read()
    print("YoloDeploy completed; checking the public version next.")


if __name__ == "__main__":
    try:
        main()
    except urllib.error.HTTPError as error:
        raise SystemExit(f"Deployment HTTP request failed with status {error.code}") from None
    except urllib.error.URLError:
        raise SystemExit("Deployment network request failed") from None
