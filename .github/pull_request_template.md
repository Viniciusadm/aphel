## What changes

<!-- Summary of the change and its motivation. Reference the issue: "Closes #123". -->

## Type

- [ ] `feat`: new feature
- [ ] `fix`: bug fix
- [ ] `chore`: maintenance, documentation, build or refactoring

## How to test

<!-- Steps for the reviewer to verify the change. -->

1.

## Screenshots

<!-- For visual changes: before and after, on desktop and mobile. Delete the section if it does not apply. -->

## Checklist

- [ ] `make lint` and `make test` pass locally
- [ ] New or updated tests cover the change (use cases with fakes, repositories with `#[sqlx::test]` including isolation between organizations, components with Vitest)
- [ ] Code in English, UI texts via i18n in pt-BR, no explanatory comments
- [ ] Business mutations publish an event and write audit in the same transaction
- [ ] New or changed routes have a `utoipa` annotation; `back/openapi.json` and `web/src/api/schema.d.ts` regenerated
- [ ] New migrations (never edit one that has already been published), if the database changes
- [ ] Documentation updated (`docs/`, `README.md`, `back/README.md`), if applicable
