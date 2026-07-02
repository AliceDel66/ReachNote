#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QA_IDENTIFIER="com.reachnote.qa"
QA_APP="$ROOT_DIR/target/debug/bundle/macos/ReachNote QA.app"
QA_DATA_DIR="$HOME/Library/Application Support/$QA_IDENTIFIER"
QA_SAVED_STATE_DIR="$HOME/Library/Saved Application State/$QA_IDENTIFIER.savedState"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "desktop-smoke-qa.sh is macOS-only because it builds and opens a .app bundle." >&2
  exit 2
fi

reset_data=false
open_app=false

for arg in "$@"; do
  case "$arg" in
    --reset-data)
      reset_data=true
      ;;
    --open)
      open_app=true
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Usage: scripts/desktop-smoke-qa.sh [--reset-data] [--open]" >&2
      exit 2
      ;;
  esac
done

cd "$ROOT_DIR"

if [[ "$reset_data" == true ]]; then
  rm -rf "$QA_DATA_DIR" "$QA_SAVED_STATE_DIR"
fi

pnpm tauri build --debug --bundles app --no-sign --config src-tauri/tauri.qa.conf.json

if [[ ! -d "$QA_APP" ]]; then
  echo "QA app bundle was not created: $QA_APP" >&2
  exit 1
fi

echo "QA app: $QA_APP"
echo "QA identifier: $QA_IDENTIFIER"
echo "QA data: $QA_DATA_DIR"

if [[ "$open_app" == true ]]; then
  open -na "$QA_APP"
fi
