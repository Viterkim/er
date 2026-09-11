#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

bash checks/core.sh
bash checks/integrations.sh

cargo run -p er --example context
cargo run -p er --example context --no-default-features --features macros
