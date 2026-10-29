# Security policy

## Versions that receive security fixes

| Version | Receives fixes |
| --- | --- |
| 0.1.x (latest) | Yes |
| Earlier than 0.1 | No |

While the project is in the 0.x series, only the latest version receives fixes. Upgrade by following [docs/upgrading.md](docs/upgrading.md).

## How to report a vulnerability

**Do not open a public issue, discussion or pull request** to report a vulnerability: that exposes Aphel users before a fix exists.

Use GitHub's private reporting: on the repository page, **Security** tab → **Report a vulnerability** (GitHub Private Vulnerability Reporting). Only the maintainers see the report.

Include, if possible:

- Aphel version (`aphel --version`) and installation method (Docker or manual);
- description of the problem and its impact (e.g. access to another organization's data, privilege escalation, code execution);
- steps to reproduce or a proof of concept;
- a suggested fix, if you have one.

Do not access, change or delete third-party data to demonstrate the problem; use your own installation (the demo data helps: `aphel demo seed --organization demo`).

## What to expect

- Acknowledgment of receipt within **5 business days**.
- Initial assessment (whether it is a vulnerability, its severity and affected versions) within **15 days**.
- A fix published in a new version, with a notice in the release notes and credit to the reporter (if they want it).
- We ask that public disclosure wait for the fix, or 90 days from the report, whichever comes first, unless otherwise agreed.

## Scope

Of particular interest:

- breaking the isolation between organizations (multi-tenancy);
- authentication, session, CSRF or authorization flaws (access beyond the role's permissions);
- SQL injection, XSS, malicious uploads, access to files without permission;
- leakage of secrets (passwords, tokens, keys) in responses, logs or exports.

Out of scope: problems that depend on an insecure configuration documented as such (e.g. running without HTTPS in production), volumetric denial-of-service attacks, and vulnerabilities in dependencies with no demonstrable impact on Aphel (report them to the dependency's project).

## Good practices for installers

- Use HTTPS (reverse proxy) and set `APP_BASE_URL` with `https://`, so the session cookie is sent with `Secure`.
- Change the passwords from `.env.example` and keep `.env` out of version control.
- Expose the Aphel port only to the proxy (`APP_PORT=127.0.0.1:3001`) and do not expose MySQL.
- Keep encrypted backups and test restoring them ([docs/backup.md](docs/backup.md)).
- Upgrade to the latest version.
