# Repository protection

Prepared settings only; no remote rule is installed automatically. Apply after
the one-time history cleanup. GitHub currently reports that rulesets for this
private repository require GitHub Pro; a public repository can use these rules.
Changing visibility is a separate owner decision.

`main-ruleset.json` requires pull requests, the GitHub Actions `check` job,
resolved conversations, verified commit signatures and linear history. It blocks
force pushes and branch deletion, with no bypass actors. Squash merging is the
only allowed merge method. Independent approvals are set to zero so a sole
maintainer can merge their own PR once checks pass; set this to one when another
reviewer is available. The `check` context is the direct PR job from `check.yml`,
not the nested `checks / check` job shown in the release workflow.

After confirming the repository can enforce rules and history is final:

```sh
gh api --method PATCH repos/chriopter/spareswarm \
  --input .github/repository/settings.json
gh api --method POST repos/chriopter/spareswarm/rulesets \
  --input .github/repository/main-ruleset.json
```

Apply once; if a ruleset exists, update its ID with `PUT` rather than creating a
duplicate. Verify the resulting rules through GitHub before relying on them.
Protect the `release` and `production` environments separately, allowing only
`main`, before adding signing or deployment credentials.

A history rewrite must use an explicit expected remote SHA with
`--force-with-lease`, after a recoverable backup. It does not erase GitHub caches,
existing clones, Actions artifacts, releases or registry images. Removing leaked
secrets from history never replaces rotating them.
