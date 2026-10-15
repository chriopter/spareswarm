import importlib.util
import pathlib
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("publish", pathlib.Path(__file__).parents[1] / "deploy/publish.py")
publish = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publish)


class DeploymentTests(unittest.TestCase):
    def env(self, url="https://example.com/deploy"):
        return patch.dict("os.environ", {"DEPLOY_WEBHOOK_URL": url,
            "DEPLOY_TOKEN": "app-token", "CF_ACCESS_CLIENT_ID": "client",
            "CF_ACCESS_CLIENT_SECRET": "secret"}, clear=True)

    def test_empty_authenticated_post(self):
        with self.env():
            request = publish.deployment_request()
        self.assertEqual(request.get_method(), "POST")
        self.assertEqual(request.get_header("User-agent"), "SpareSwarm-Deploy/1.0")
        self.assertEqual(request.data, b"")
        self.assertEqual(request.get_header("Authorization"), "Bearer app-token")
        self.assertEqual(request.get_header("Cf-access-client-secret"), "secret")

    def test_rejects_ambiguous_or_insecure_urls(self):
        for url in ("http://example.com/deploy", "https://user@example.com/deploy",
                    "https://example.com/deploy?repo=other", "https://example.com/other",
                    "https://example.com/deploy#fragment"):
            with self.subTest(url=url), self.env(url), self.assertRaises(ValueError):
                publish.deployment_request()

    def test_redirects_do_not_forward_secrets(self):
        self.assertIsNone(publish.NoRedirect().redirect_request(None, None, 302, "", {}, "https://other.com"))


if __name__ == "__main__":
    unittest.main()
