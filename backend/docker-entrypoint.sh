#!/bin/sh
set -e

# Railway (and any platform that provisions a fresh persistent volume) mounts
# the volume owned by root, shadowing the build-time ownership of AGENT_MAKER_HOME.
# The app runs unprivileged, so make the data dir writable before dropping
# privileges. This is a no-op when the dir is already owned correctly (e.g. local
# Docker named volumes, which inherit the image's ownership on first mount).
DATA_DIR="${AGENT_MAKER_HOME:-/data}"
mkdir -p "$DATA_DIR"
chown -R appuser:appuser "$DATA_DIR" 2>/dev/null || true

# Drop from root to the unprivileged runtime user and exec the app.
exec gosu appuser "$@"
