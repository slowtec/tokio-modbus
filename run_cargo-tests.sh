#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
# SPDX-License-Identifier: CC0-1.0

set -euo pipefail

# Change into directory where this shell script is located
SCRIPT_ROOT=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd "${SCRIPT_ROOT}"

cargo test --no-default-features
cargo test --no-default-features --all-targets --features rtu
cargo test --no-default-features --all-targets --features tcp
cargo test --no-default-features --all-targets --features rtu-sync
cargo test --no-default-features --all-targets --features tcp-sync
cargo test --no-default-features --all-targets --features rtu-server
cargo test --no-default-features --all-targets --features tcp-server
cargo test --no-default-features --all-targets --features rtu-over-tcp-server
cargo test --all-features --all-targets
