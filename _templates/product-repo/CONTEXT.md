---
type: product-repo-template
purpose: "Boilerplate for a new product repository consuming HeavenOS as a git submodule"
---

# Product repo template — consuming HeavenOS as a submodule

Per the multi-repo strategy: `HeavenOS` stays a pure core repo — no
product-specific code lands here. Every product gets its own repository,
and pulls `HeavenOS` in as a git submodule pinned to a **tagged release**,
never `main` directly. Root [`README.md`](../../README.md#using-heavenos-in-a-product)
has the short version; this is the full procedure and the reasoning behind
it.

## Why tagged releases, not `main`

A submodule is always pinned to one exact commit SHA — there is no
git-level "track this branch" for a submodule. Pinning to whatever commit
happened to be `main`'s HEAD when you ran `git submodule add` means every
subsequent `HeavenOS` commit is a candidate for the next Dependabot bump,
including ones that were never meant as a stable checkpoint. Pinning to a
tagged commit instead gives a deliberate release boundary: `HeavenOS`
keeps moving on `main`, and nothing downstream sees a given change until a
new tag says it's ready.

## How Dependabot actually resolves this — verified, not assumed

Dependabot's `gitsubmodule` ecosystem fetches up to 250 commits from the
submodule's tracked branch and looks for commits carrying a valid semver
tag (`v` prefix optional). **Tagged commits are treated as release
versions and take priority over newer, untagged commits** when Dependabot
proposes a bump — this shipped in `dependabot-core` (PR #13052, confirmed
deployed February 2026) and needs zero special `dependabot.yml`
configuration to activate. Concretely: as long as `HeavenOS`'s `main`
carries `v*.*.*` tags, Dependabot keeps proposing the *latest tagged*
commit, not the latest raw commit on `main` — exactly the
deliberate-release-boundary behaviour this strategy wants, using
Dependabot's stock behaviour rather than anything hand-rolled here.

## Setup, step by step

1. Create the new (empty) product repository on GitHub.

2. Add `HeavenOS` as a submodule, then move its pin to the tag you want —
   `git submodule add` always checks out the default branch first; you
   move the pin to a tag afterward:

   ```bash
   git submodule add https://github.com/Thinkman405/HeavenOS.git vendor/heavenos
   cd vendor/heavenos
   git checkout v0.1.0    # or whichever tag you're starting from
   cd ../..
   git add vendor/heavenos
   git commit -m "Add HeavenOS v0.1.0 as a submodule"
   ```

   `vendor/heavenos` is a convention, not a rule — pick whatever path
   makes sense for the product repo.

3. Copy [`dependabot.yml`](dependabot.yml) from this template into
   `.github/dependabot.yml` in the new repo, unchanged.

4. Push. Dependabot picks the config up automatically (daily interval); it
   opens a PR bumping the submodule pointer whenever `HeavenOS` cuts a new
   tag, and that PR runs the product repo's own CI before you merge it —
   no long-lived branch, no manual tracking.

## Cutting a new HeavenOS release, for the core side

Push a tag matching `v*.*.*` to `HeavenOS`'s `main`.
[`.github/workflows/release.yml`](../../.github/workflows/release.yml)
builds and tests the workspace one more time, then creates a GitHub
Release automatically via `gh release create`. There is no fixed cadence
or version-bump rule beyond "this is a real, stable checkpoint" — a
judgement call, stated as one, not a formal semver policy this project has
committed to.
