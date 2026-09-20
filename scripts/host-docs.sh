#!/usr/bin/env bash
set -euo pipefail

echo ""
echo "do the:"
echo "ssh -L 8585:127.0.0.1:8585 user@xxx.xxx.xxx"
echo ""

cargo doc -p er --all-features --no-deps; python3 -m http.server 8585 --directory target/doc --bind 127.0.0.1

