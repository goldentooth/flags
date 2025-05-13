#!/usr/bin/env bash

: "${1?"Usage: ${FUNCNAME[0]} NUMBER_OF_NODES"}";

set -euo pipefail;

num_nodes=$1;
if [ "$num_nodes" -lt 1 ]; then
  echo "Number of nodes must be at least 1";
  exit 1;
fi;

cd "$(dirname $0)";

cargo build --release --bin whispers;

for node_id in $(seq 1 "${num_nodes}"); do
    (RUST_LOG=debug cargo run --release -- 2>&1 | sed -e "s/\\(.*\\)/whispers $node_id \\1/g") &
done;

trap 'kill $(jobs -p)' EXIT;
wait < <(jobs -p);
