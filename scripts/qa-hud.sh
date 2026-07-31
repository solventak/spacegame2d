#!/usr/bin/env bash
set -euo pipefail

hud_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../crates/spacegame2d/hud" && pwd)"
cd "$hud_dir"
npm ci
npm run contract:check
npm run check
npm test
npm run build
git -C "$hud_dir/../../.." diff --exit-code -- crates/spacegame2d/hud/dist
