# Security Policy

The threat model, the P2.6 review, and the current findings and residual risks are recorded in [docs/security/SECURITY_REVIEW.md](docs/security/SECURITY_REVIEW.md).

## Supported versions

Bluewhale is pre-alpha. Only the `main` branch and the latest tagged pre-release receive security fixes. Older commits and forks are not supported.

## Reporting a vulnerability

Do not open a public issue for a security problem.

Use GitHub's private vulnerability reporting for this repository: open the **Security** tab and choose **Report a vulnerability**. Include:

- The affected component (core crate, desktop client, Android client, or server) and version or commit
- The impact and the conditions required to reproduce it
- Reproduction steps or a minimal proof of concept
- Any suggested fix or mitigation

Never include real financial data, vault keys, encryption keys, recovery keys, password material, or ciphertext from a real ledger. Use synthetic data in reports and screenshots. If a report accidentally contains a secret, say so in the advisory so the secret can be rotated.

## Response expectations

- Acknowledgement within 3 business days.
- Initial assessment and severity within 10 business days.
- A coordinated disclosure date agreed with the reporter after a fix is available.
- Credit in the advisory unless the reporter asks to remain anonymous.

## Scope

In scope: the public core crates and clients, the cryptographic key hierarchy, the sync protocol, local SQLCipher storage, installer and update integrity, and the private server where it affects client security.

Out of scope: vulnerabilities in third-party dependencies that are already fixed upstream (report them upstream, and open a private advisory if Bluewhale must upgrade), denial of service against a self-hosted instance with no security impact, and findings that require an already-compromised device.

## Safe harbor

Good-faith security research that follows this policy, avoids privacy violations, avoids destructive testing, and does not exfiltrate data will not be pursued legally. Do not test against infrastructure or accounts you do not own or have permission to test.

## Disclosure

After a fix is released, the advisory describes the affected versions and the mitigation. Security fixes are noted in `CHANGELOG.md` under **Security** without technical detail that would endanger users who have not upgraded yet.
