#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

GIT_ROOT_DIR="$(git rev-parse --show-toplevel)"

BANYAN_CORE_ROOT_DIR="${GIT_ROOT_DIR}/../banyan-core"
BANYAN_CORE_FRONTEND_DIR="${BANYAN_CORE_ROOT_DIR}/crates/banyan-core-service/frontend"
INLINED_TOMB_DIR="${BANYAN_CORE_FRONTEND_DIR}/tomb_build"

PKG_NAME="tomb-wasm-experimental"

# We need to link our inlined builds as they have a matching name with our frontend
(cd ${INLINED_TOMB_DIR} && (yarn unlink || true) && yarn link)
(cd ${BANYAN_CORE_FRONTEND_DIR} && (yarn unlink ${PKG_NAME} || true) && yarn link ${PKG_NAME})
