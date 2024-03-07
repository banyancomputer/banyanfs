#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail

DEBUG=${DEBUG:-1}

GIT_ROOT_DIR="$(git rev-parse --show-toplevel)"

BANYAN_CORE_ROOT_DIR="${GIT_ROOT_DIR}/../banyan-core"
BANYAN_CORE_FRONTEND_DIR="${BANYAN_CORE_ROOT_DIR}/crates/banyan-core-service/frontend"
INLINED_TOMB_DIR="${BANYAN_CORE_FRONTEND_DIR}/tomb_build"

PACK_DEBUG_FLAG=""
if [ ${DEBUG} -eq 1 ]; then
	PACK_DEBUG_FLAG="--debug"
fi

(cd ${GIT_ROOT_DIR} && wasm-pack build ${PACK_DEBUG_FLAG})

rm -rf ${BANYAN_CORE_FRONTEND_DIR}/node_modules
rm -rf ${INLINED_TOMB_DIR}/*

cp -f pkg/* ${INLINED_TOMB_DIR}/
(cd ${INLINED_TOMB_DIR} && jq '.name = "tomb-wasm-experimental"' package.json >tmp.$$.json && mv tmp.$$.json package.json)
(cd ${BANYAN_CORE_FRONTEND_DIR} && yarn install)

# Probably don't want to keep this around permanently, but its convenient for now
(cd ${BANYAN_CORE_ROOT_DIR} && rm -f dist/assets/*)

FRONTEND_DEBUG_FLAG=""
if [ ${DEBUG} -eq 1 ]; then
	FRONTEND_DEBUG_FLAG="--mode development"
fi

(cd ${BANYAN_CORE_FRONTEND_DIR} && yarn build ${FRONTEND_DEBUG_FLAG})
