# Contributing

Thanks for your interest in contributing to drivers!

> This repository is a public mirror. Changes are reviewed here, then imported
> into the maintainers' internal repository, validated against the canonical CI,
> and merged — after which the merged change is mirrored back to this repo.

## How to contribute

1. **Fork** this repository and create a branch for your change.
2. **Make your change**, following the conventions of the code you're editing
   (see [`README.md`](README.md) and the per-language directories for build and
   test setup).
3. **Run the tests** before opening a PR — see [Running CI](#running-ci) below.
4. **Open a pull request** against this repository. Keep it focused, and
   describe what changed and why.

## Running CI

You can validate your change two ways:

- **Locally** — see [`README.md`](README.md) ("Running Tests") for building the
  components and setting up credentials.
- **In your fork's GitHub Actions**, against your own Snowflake account — the
  full per-PR test suite runs on your own infrastructure so you get the full
  signal before opening a PR. See
  **[Running CI in your fork](docs/running-ci-in-your-fork.md)**.

## Review and merge

A maintainer reviews your pull request. Once it looks good, they import it into
the internal repository (via an `ok-to-import` label) so it runs against the
canonical internal CI before merge. When merged, the change is mirrored back to
this repository and your original PR is closed with a link to the merged commit.
