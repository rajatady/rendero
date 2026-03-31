#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${PORT:-5556}"
OUT_DIR="${OUT_DIR:-/tmp/rendero-verify-$(date +%Y%m%d-%H%M%S)}"
SERVER_LOG="${OUT_DIR}/serve.log"
NATIVE_LOG="${OUT_DIR}/native.log"
NATIVE_PPM="${OUT_DIR}/native-headless.ppm"
NATIVE_PNG="${OUT_DIR}/native-headless.png"
NATIVE_JSON="${OUT_DIR}/native-headless.json"

mkdir -p "${OUT_DIR}"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "${SERVER_PID}" 2>/dev/null || true
  fi
  if [[ -n "${NATIVE_PID:-}" ]]; then
    kill "${NATIVE_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

echo "[1/6] Building DOM shim bundles"
(cd "${ROOT}/docs/demos/dom-shim" && npm run build)

echo "[2/6] Building browser WASM package into docs/pkg"
(cd "${ROOT}" && wasm-pack build crates/wasm --target web --out-dir ../../docs/pkg --out-name rendero)

echo "[3/6] Building native shell"
(cd "${ROOT}" && cargo build -p rendero-native-shell)

echo "[4/6] Starting no-cache static server on :${PORT}"
serve -l "${PORT}" docs -C --no-etag -c "${ROOT}/serve.rendero.json" >"${SERVER_LOG}" 2>&1 &
SERVER_PID=$!
sleep 2

echo "[5/6] Capturing browser screenshots with Playwright"
python3 "${ROOT}/scripts/capture_dom_shim.py" "http://localhost:${PORT}" "${OUT_DIR}"

echo "[6/6] Running native headless dump and native app"
(cd "${ROOT}" && \
  RENDERO_DEMO=react \
  RENDERO_HEADLESS_DUMP="${NATIVE_PPM}" \
  RENDERO_HEADLESS_METADATA="${NATIVE_JSON}" \
  RENDERO_HEADLESS_WIDTH=1440 \
  RENDERO_HEADLESS_HEIGHT=3400 \
  cargo run -p rendero-native-shell >"${NATIVE_LOG}" 2>&1)

if command -v sips >/dev/null 2>&1 && [[ -f "${NATIVE_PPM}" ]]; then
  sips -s format png "${NATIVE_PPM}" --out "${NATIVE_PNG}" >/dev/null
fi

echo
echo "Verification artifacts written to ${OUT_DIR}"
echo "  Browser React Web:      ${OUT_DIR}/react-web.png"
echo "  Browser React Rendero:  ${OUT_DIR}/react-rendero.png"
echo "  Browser state:          ${OUT_DIR}/browser-state.json"
echo "  Browser console:        ${OUT_DIR}/browser-console.json"
echo "  Native headless PPM:    ${NATIVE_PPM}"
echo "  Native headless PNG:    ${NATIVE_PNG}"
echo "  Native headless JSON:   ${NATIVE_JSON}"
echo "  Native log:             ${NATIVE_LOG}"
echo "  Server log:             ${SERVER_LOG}"
