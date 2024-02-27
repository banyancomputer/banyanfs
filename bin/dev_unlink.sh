#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

GIT_ROOT_DIR="$(git rev-parse --show-toplevel)"

BANYAN_CORE_ROOT_DIR="${GIT_ROOT_DIR}/../banyan-core"
BANYAN_CORE_DIST_ASSETS_DIR="${BANYAN_CORE_ROOT_DIR}/crates/banyan-core-service/dist/assets"
BANYAN_CORE_FRONTEND_DIR="${BANYAN_CORE_ROOT_DIR}/crates/banyan-core-service/frontend"
INLINED_TOMB_DIR="${BANYAN_CORE_FRONTEND_DIR}/tomb_build"

(cd ${INLINED_TOMB_DIR} && (yarn unlink || true))

# Remove any lingering copies of the prototype wasm builds
rm -f ${BANYAN_CORE_DIST_ASSETS_DIR}/*
(cd ${BANYAN_CORE_FRONTEND_DIR} && rm -rf node_modules && yarn install && yarn build)
