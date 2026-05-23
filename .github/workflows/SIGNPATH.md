# SignPath Code Signing — Onboarding

DLSSync uses SignPath.io for Authenticode code signing of Windows release artifacts. SignPath sponsors signing for open-source projects at no cost.

This document describes how to bring signing online after the first GitHub release. Without it, releases are still produced and the auto-updater works, but Windows SmartScreen will warn the user on first run.

## Why this matters

- Tauri's updater verifies an Ed25519 signature on each release. That protects against tampering of the update payload itself. The key pair lives in TAURI_SIGNING_PRIVATE_KEY (GitHub secret) and plugins.updater.pubkey in src-tauri/tauri.conf.json.
- Windows SmartScreen verifies an Authenticode signature on the executable. That is what makes the "Unknown publisher" warning go away, and that is what SignPath provides.

Both are required for a friction-free production release. Ed25519 is configured in the workflow; SignPath is the missing piece a maintainer enables once.

## Step 1 — Apply to the OSS program

1. Open signpath.io/product/open-source.
2. Click "Apply for free".
3. Submit with the repo URL https://github.com/xt0n1-t3ch/DLSSync, project description, and the reason ("Authenticode signing for SmartScreen reputation on Windows releases").
4. Approval typically lands in 1-3 business days.

## Step 2 — Configure the SignPath project

Once approved, in the SignPath dashboard:

1. Create a project and link it to xt0n1-t3ch/DLSSync on GitHub.
2. Create a signing policy named release-signing for production builds.
3. Install the SignPath GitHub App on the repo so SignPath can read workflow provenance.
4. Create an API token with submitter permissions for the policy.

## Step 3 — Add GitHub secrets

In https://github.com/xt0n1-t3ch/DLSSync/settings/secrets/actions, add:

- SIGNPATH_API_TOKEN — API token from the SignPath dashboard.
- SIGNPATH_ORG_ID — Organization ID from the SignPath dashboard.
- TAURI_SIGNING_PRIVATE_KEY — Ed25519 private key for the updater (generation steps below).
- TAURI_SIGNING_PRIVATE_KEY_PASSWORD — Password for the Ed25519 key.

### Generate the Tauri updater key pair

Run locally once to create the Ed25519 key pair the auto-updater uses to verify update payloads:

```
pnpm tauri signer generate -w $env:USERPROFILE\.tauri\dlssync.key
```

This emits the private key on disk (used as TAURI_SIGNING_PRIVATE_KEY secret) and prints the matching public key to stdout. Paste the public key into src-tauri/tauri.conf.json at plugins.updater.pubkey.

Treat the private key like a master password. A compromised key lets an attacker push a malicious update to every installed copy.

## Step 4 — Enable signing in the release workflow

The release workflow at .github/workflows/release.yml contains an opt-in sign-windows job gated by `if: vars.SIGNPATH_ENABLED == 'true'`. To switch it on, set the repository variable SIGNPATH_ENABLED=true in https://github.com/xt0n1-t3ch/DLSSync/settings/variables/actions.

Until that variable is true, the workflow builds and publishes unsigned artifacts so the first releases can ship while the SignPath application is in review.

## Step 5 — Verify after the first signed release

1. Download the published executable from the GitHub release.
2. Right-click the file, then Properties, then the Digital Signatures tab.
3. The signature should appear as valid, signed by SignPath on behalf of the project.
4. Run the file. SmartScreen should not show an "Unknown publisher" warning. Reputation builds over the first few hundred downloads; mature OSS projects on SignPath rarely see warnings beyond that point.

## References

- SignPath docs: https://docs.signpath.io/
- SignPath OSS program: https://signpath.io/product/open-source
- GitHub Action used: https://github.com/signpath/github-action-submit-signing-request
- Tauri Windows signing guide: https://v2.tauri.app/distribute/sign/windows/
