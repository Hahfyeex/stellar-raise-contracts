# deployment_shell_script.sh

Builds, deploys, and initialises the Stellar Raise crowdfund contract with
structured error capturing and timestamped logging.

## Why this script exists

The original `deploy.sh` used `set -e` but swallowed error context — a failed
`cargo build` or `stellar contract deploy` would exit silently with no
actionable message. This script adds:

- Per-step exit codes (2–5) so CI can distinguish build vs deploy vs init failures.
- All stderr captured to `DEPLOY_LOG` (default `deploy_errors.log`) alongside
  timestamped stdout entries.
- Argument validation with clear messages before any network call is made.

## Usage

```bash
./scripts/deployment_shell_script.sh <creator> <token> <goal> <deadline> [min_contribution]
```

| Parameter         | Type    | Description                                      |
| :---------------- | :------ | :----------------------------------------------- |
| `creator`         | string  | Stellar address of the campaign creator          |
| `token`           | string  | Stellar address of the token contract            |
| `goal`            | integer | Funding goal in stroops                          |
| `deadline`        | integer | Unix timestamp — must be in the future           |
| `min_contribution`| integer | Minimum pledge amount (default: `1`)             |

### Environment variables

| Variable     | Default            | Description                        |
| :----------- | :----------------- | :--------------------------------- |
| `NETWORK`    | `testnet`          | Stellar network to target          |
| `DEPLOY_LOG` | `deploy_errors.log`| File that captures all error output|

### Example

```bash
DEADLINE=$(date -d "+30 days" +%s)
./scripts/deployment_shell_script.sh \
  GCREATOR... GTOKEN... 1000 "$DEADLINE" 10
```

## Exit codes

| Code | Meaning                        |
| :--- | :----------------------------- |
| 0    | Success                        |
| 1    | Missing dependency (cargo / stellar CLI) |
| 2    | Invalid or missing argument    |
| 3    | `cargo build` failure          |
| 4    | `stellar contract deploy` failure |
| 5    | `stellar contract invoke` (init) failure |

## Error log format

Every line written to `DEPLOY_LOG` follows:

```
[2026-03-23T16:00:00Z] [INFO|WARN|ERROR] <message>
```

Stderr from `cargo` and `stellar` is appended verbatim after the tagged line,
making it straightforward to `grep` for specific failures in CI logs.

## Security assumptions

- The `creator` argument is used as both the signing source and the on-chain
  creator address. Never pass a raw secret key; use a named Stellar CLI identity.
- `DEPLOY_LOG` may contain sensitive RPC responses. Restrict file permissions
  in production (`chmod 600 deploy_errors.log`).
- The script does **not** store or echo secret keys at any point.
- `set -euo pipefail` ensures unhandled errors abort execution immediately.

## Running the tests

```bash
bash scripts/deployment_shell_script.test.sh
```

No external test framework is required. The test file stubs `cargo` and
`stellar` so the suite runs offline and in CI without network access.

### Test coverage

| Area                        | Cases |
| :-------------------------- | :---- |
| `require_tool`              | 2     |
| `validate_args`             | 9     |
| `build_contract`            | 3     |
| `deploy_contract`           | 3     |
| `init_contract`             | 2     |
| `log` / `die`               | 4     |
| `DEPLOY_LOG` file behaviour | 2     |
| **Total**                   | **25**|
