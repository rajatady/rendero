#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${PORT:-5555}"
HOST="${HOST:-127.0.0.1}"
OUT_DIR="${OUT_DIR:-${ROOT}/accuracy}"
URL="${URL:-http://${HOST}:${PORT}/demos/dom-shim/}"
SERVER_LOG="${OUT_DIR}/serve.log"
SUMMARY_JSON="${OUT_DIR}/summary.json"
NATIVE_JSON="${OUT_DIR}/apple-native.json"
RUNTIME_COMPARISON_JSON="${OUT_DIR}/apple-runtime-comparison.json"

mkdir -p "${OUT_DIR}"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

echo "[1/8] Build DOM-shim bundles"
(cd "${ROOT}/docs/demos/dom-shim" && node build.mjs)

echo "[2/8] Build WASM package into docs/pkg"
(cd "${ROOT}" && wasm-pack build crates/wasm --target web --out-dir ../../docs/pkg --out-name rendero)

echo "[3/8] Refresh corpus dashboard"
(cd "${ROOT}" && python3 corpus/dashboard.py)

echo "[4/8] Start no-cache dev server on ${HOST}:${PORT}"
cd "${ROOT}"
serve -l "tcp://${HOST}:${PORT}" docs -C --no-etag -c "${ROOT}/serve.rendero.json" >"${SERVER_LOG}" 2>&1 &
SERVER_PID=$!
sleep 2

echo "[5/8] Capture browser oracle"
(cd "${ROOT}" && python3 scripts/capture-ground-truth.py "${URL}" "${OUT_DIR}/apple-web.json")

echo "[6/10] Capture Rendero engine layout"
(cd "${ROOT}" && python3 scripts/capture-engine-truth.py "${URL}" "${OUT_DIR}/apple-engine.json")

echo "[7/10] Capture native Rendero headless layout"
(cd "${ROOT}" && python3 scripts/capture-native-truth.py "${NATIVE_JSON}" 1440 900)

echo "[8/10] Compare browser oracle vs browser Rendero"
set +e
(cd "${ROOT}" && python3 scripts/compare-layout.py "${OUT_DIR}/apple-web.json" "${OUT_DIR}/apple-engine.json" "${OUT_DIR}/apple-comparison.json")
COMPARE_EXIT=$?
set -e

echo "[9/10] Compare browser, browser Rendero, and native together"
set +e
(cd "${ROOT}" && python3 scripts/compare-runtime-triple.py "${OUT_DIR}/apple-web.json" "${OUT_DIR}/apple-engine.json" "${NATIVE_JSON}" "${RUNTIME_COMPARISON_JSON}")
TRIPLE_COMPARE_EXIT=$?
set -e

echo "[10/10] Run synthetic layout corpus benchmark"
(cd "${ROOT}" && python3 scripts/capture-layout-corpus.py "http://${HOST}:${PORT}" "${OUT_DIR}/layout-corpus.json")

python3 - "${ROOT}" "${OUT_DIR}" "${SUMMARY_JSON}" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
out_dir = Path(sys.argv[2])
summary_path = Path(sys.argv[3])

comparison = json.loads((out_dir / "apple-comparison.json").read_text())
runtime_comparison = json.loads((out_dir / "apple-runtime-comparison.json").read_text())
dashboard = json.loads((root / "corpus" / "dashboard.json").read_text())
layout_corpus = json.loads((out_dir / "layout-corpus.json").read_text())
layout_property_matches = layout_corpus.get("propertyMatches", layout_corpus.get("matchCount", 0))
layout_total_properties = layout_corpus.get("totalProperties", layout_corpus.get("total", 0))

summary = {
    "appleAccuracy": comparison["accuracy"],
    "appleMatchCount": comparison["matchCount"],
    "appleTotalProperties": comparison["totalProperties"],
    "appleElementsCompared": comparison["elementsPaired"],
    "appleElementsWithMismatches": comparison["elementsWithMismatches"],
    "nativeAccuracy": runtime_comparison["pairwise"]["browserVsNative"]["accuracy"],
    "nativeMatchCount": runtime_comparison["pairwise"]["browserVsNative"]["matchCount"],
    "nativeTotalProperties": runtime_comparison["pairwise"]["browserVsNative"]["totalProperties"],
    "wasmVsNativeAccuracy": runtime_comparison["pairwise"]["wasmVsNative"]["accuracy"],
    "layoutCorpusAccuracy": round((layout_property_matches / max(layout_total_properties, 1)) * 100, 2),
    "layoutCorpusPropertyMatches": layout_property_matches,
    "layoutCorpusTotalProperties": layout_total_properties,
    "layoutCorpusTestCases": len(layout_corpus["testCases"]),
    "corpusSiteCount": dashboard["corpusSummary"]["siteCount"],
    "corpusGroundTruthFiles": dashboard["corpusSummary"]["totalGroundTruthFiles"],
    "corpusTotalElementsDesktop": dashboard["corpusSummary"]["totalElements"],
    "featureCoverageAcrossSites": dashboard["featureCoverageAcrossSites"],
}

summary_path.write_text(json.dumps(summary, indent=2))
print(f"Summary: {summary_path}")
print(f"Apple accuracy: {summary['appleAccuracy']}% ({summary['appleMatchCount']}/{summary['appleTotalProperties']})")
print(f"Native accuracy: {summary['nativeAccuracy']}% ({summary['nativeMatchCount']}/{summary['nativeTotalProperties']})")
print(f"WASM vs native accuracy: {summary['wasmVsNativeAccuracy']}%")
print(f"Synthetic corpus accuracy: {summary['layoutCorpusAccuracy']}% ({summary['layoutCorpusPropertyMatches']}/{summary['layoutCorpusTotalProperties']})")
print(f"Corpus: {summary['corpusSiteCount']} sites, {summary['corpusGroundTruthFiles']} ground-truth files")
PY

echo
echo "Artifacts:"
echo "  Browser oracle:   ${OUT_DIR}/apple-web.json"
echo "  Engine capture:   ${OUT_DIR}/apple-engine.json"
echo "  Comparison:       ${OUT_DIR}/apple-comparison.json"
echo "  Runtime comparison: ${RUNTIME_COMPARISON_JSON}"
echo "  Native capture:   ${NATIVE_JSON}"
echo "  Layout corpus:    ${OUT_DIR}/layout-corpus.json"
echo "  Summary:          ${SUMMARY_JSON}"
echo "  Browser screenshot: ${OUT_DIR}/apple-web.png"
echo "  Engine screenshot:  ${OUT_DIR}/apple-engine.png"
echo "  Native screenshot:  ${OUT_DIR}/apple-native.png"
echo "  Corpus screenshot:  ${OUT_DIR}/layout-corpus.png"
echo "  Corpus dashboard: ${ROOT}/corpus/dashboard.json"
echo "  Server log:       ${SERVER_LOG}"

if [[ "${COMPARE_EXIT}" -ne 0 || "${TRIPLE_COMPARE_EXIT}" -ne 0 ]]; then
  exit 1
fi

exit 0
