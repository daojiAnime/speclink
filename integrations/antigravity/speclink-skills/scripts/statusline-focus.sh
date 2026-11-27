#!/usr/bin/env bash
# Print the active Speclink focus segment (e.g. `speclink: apply · my-change`)
# from Antigravity CLI statusLine/title JSON on stdin (or `$1` as conversationId).
set -uo pipefail

if [ -n "${1:-}" ]; then
  conv_id="$1"
elif command -v jq >/dev/null 2>&1; then
  conv_id="$(jq -r '.conversation_id // .session_id // empty' 2>/dev/null || true)"
else
  conv_id=""
fi

[ -z "$conv_id" ] && exit 0
safe_id="$(printf '%s' "$conv_id" | tr -c 'A-Za-z0-9._-' '_' | cut -c1-120)"
[ -z "$safe_id" ] && exit 0

focus_file="${TMPDIR:-/tmp}/speclink-agy-focus/${safe_id}.json"
[ -f "$focus_file" ] || exit 0
command -v jq >/dev/null 2>&1 || exit 0

skill="$(jq -r '.skill // empty' "$focus_file" 2>/dev/null | sed 's/^speclink-//')"
change="$(jq -r '.change // empty' "$focus_file" 2>/dev/null)"
discussion="$(jq -r '.discussion // empty' "$focus_file" 2>/dev/null)"

target="${change:-$discussion}"
if [ -n "$skill" ] && [ -n "$target" ]; then
  printf '%s · %s\n' "$skill" "$target"
elif [ -n "$skill" ]; then
  printf '%s\n' "$skill"
elif [ -n "$target" ]; then
  printf '%s\n' "$target"
fi
