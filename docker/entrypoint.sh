#!/bin/sh
set -eu
storage_dir="${APOLO_STORAGE_DIR:-/data/apolo}"
mkdir -p "$storage_dir"
chown apolo:apolo "$storage_dir"
exec gosu apolo /usr/local/bin/apolo-ax-api
