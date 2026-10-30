# Security policy

Only the newest beta receives security fixes. This project currently provides attended, view-only screen sharing; it does not provide account authentication, unattended access, remote input, or tenant isolation in the live API.

Report a suspected vulnerability privately through [GitHub's security advisory form](https://github.com/dengexco/dengex-desk/security/advisories/new). Include the affected version, prerequisites, impact and a minimal reproduction without real invitation codes, credentials, names, SDP, or screen captures. Do not place exploitable details or secrets in a public issue. There is no guaranteed response SLA or bounty program.

The HTTPS server is a trusted authority: it holds the session signing key and can issue grants. Use your own server or an operator you trust. Endpoints verify the TLS certificate and do not follow redirects. A display name is self-asserted, not an authenticated identity. Keep the attended consent prompt and visible stop control enabled.

Invitation consumption, grant verification, expiry, anti-replay, bounded requests, rate limits, DLL integrity/loading and relay-only behavior are covered by automated tests. Tests do not replace native two-device acceptance or a professional audit. Desktop releases are not Authenticode-signed or Apple-notarized. DLL hash pinning detects a mismatched DLL but cannot defend against replacement of both EXE and DLL.

See [deployment boundaries](docs/self-hosting.md), [DLL design](docs/security-and-dll.md) and [test coverage](docs/testing/beta-release.md).
