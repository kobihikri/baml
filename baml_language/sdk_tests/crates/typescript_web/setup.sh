#!/usr/bin/env bash
# Browser TypeScript setup for sdk_test_typescript_web.
# Invoked automatically by `cargo nextest run` through .config/nextest.toml.
set -euo pipefail

cd "$(dirname "$0")"
WORKSPACE_ROOT="$(cd ../../.. && pwd)"
BRIDGE_WEB="$WORKSPACE_ROOT/sdks/web/bridge_web"
export npm_config_store_dir="$WORKSPACE_ROOT/target/pnpm-store"
mkdir -p "$npm_config_store_dir"

(cd "$BRIDGE_WEB" && pnpm install --ignore-workspace --ignore-scripts)
(cd "$BRIDGE_WEB" && pnpm build:debug)

playwright_fixture=""
for fixture_dir in */generated; do
    [[ -d "$fixture_dir" ]] || continue
    (cd "$fixture_dir" && pnpm install --force --ignore-workspace --ignore-scripts)
    (cd "$fixture_dir" && pnpm update @boundaryml/baml-bridge-web --force --ignore-workspace --ignore-scripts)
    if [[ -z "$playwright_fixture" ]]; then
        playwright_fixture="$fixture_dir"
    fi
done

# Match the existing webview browser-test workflow: use Playwright-managed
# Chromium rather than relying on a system Google Chrome installation. Every
# fixture pins the same Playwright version, so installing from one is enough.
if [[ -n "$playwright_fixture" ]]; then
    (cd "$playwright_fixture" && pnpm exec playwright install chromium)
fi

if [[ -n "${NEXTEST_ENV:-}" ]]; then
    echo "SDK_TEST_TYPESCRIPT_WEB_SETUP=1" >> "$NEXTEST_ENV"
fi
