#!/bin/sh
set -e
python3 -c "open('/tmp/opencode/big.log','w').write(''.join(f'2026-01-01T00:00:00Z line {i}\n' for i in range(50000)))"
echo "generated 50000 line log at /tmp/opencode/big.log"
cargo build --release
/usr/bin/time -v ./target/release/runline --help 2>&1 | head -n 5 || ./target/release/runline --help
