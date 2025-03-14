#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright (c) 2017-2025 slowtec GmbH <post@slowtec.de>
# SPDX-License-Identifier: CC0-1.0

set -euo pipefail

# Change into directory where this shell script is located
SCRIPT_ROOT=$(cd -P -- "$(dirname -- "$0")" && pwd -P)
cd "${SCRIPT_ROOT}"

NIGHTLY_TOOLCHAIN=nightly

cargo +${NIGHTLY_TOOLCHAIN} udeps --all-targets --backend depinfo
