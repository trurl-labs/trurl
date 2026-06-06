# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| latest release | Yes |
| older releases | No |

Only the latest release receives security updates.

## Reporting a Vulnerability

**Do not open a public issue.**

1. **Preferred:** use [GitHub Private Vulnerability Reporting](https://github.com/trurl-labs/trurl/security/advisories/new).
2. **Fallback:** email security@trurl.dev with subject `[Trurl Security]`.

Include:
- Description of the vulnerability and its impact
- Reproduction steps or proof of concept
- Affected version(s) and component(s)

## Response Timeline

| Step | Target |
|------|--------|
| Acknowledgement | 48 hours |
| Triage and severity assessment | 5 business days |
| Fix | Dependent on severity |
| Public disclosure | After fix is released |

## Coordinated Disclosure

We follow coordinated disclosure. We ask reporters to keep findings confidential until a fix is released. We credit reporters in the release notes unless anonymity is requested.

## Safe Harbor

We consider security research conducted in good faith to be authorized and will not pursue legal action against researchers who:

- Make a good-faith effort to avoid privacy violations, data destruction, and service disruption
- Do not exploit a vulnerability beyond the minimum necessary to demonstrate it
- Report the vulnerability through the channels described above before any public disclosure

## Scope

### In scope

- **Decision store:** `.trurl/` file operations, atomic writes, validation, file locking
- **MCP server:** `trurl serve` — decision retrieval, response assembly, protocol handling
- **Conversational AI:** `trurl design` — API key handling, session state management
- **Map server:** `trurl map` — local web server, API endpoints, file system access
- **CLI:** all subcommands — file system operations, input validation

### Out of scope

- Example configurations and templates
- Third-party AI coding agents consuming Trurl's MCP output
- The content of user-authored decisions (that's your architecture, not ours)
