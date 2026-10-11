#!/bin/bash
# scripts/docs_gates.sh — every documentation gate, run against one checkout.
#
# Usage: scripts/docs_gates.sh <worktree> [--examples] [--qa] [--figures] [--strict]
#   The orchestrator's tools/docs_gates.sh delegates here with the identical invocation
#   (`scripts/docs_gates.sh <worktree> [--examples]`), so nothing else in the harness changes.
#
# Gates (one line each; the last line is "== RESULT: N failed gate(s)"; exit code = N):
#   1   CI=true mkdocs build --strict            (a missing nav page, <img src> or anchor is fatal)
#   2   scripts/check_docs_math.py               over docs/, README.md, CONTRIBUTING.md
#   2b  scripts/check_docs_badges.py docs/ README.md   (tolerant unless HUV_BADGELINT_STRICT=1;
#                                                "skip (script absent)" until the lint lands)
#   2c  the CI Status-line grep                  (every non-index page outside the narrative trees)
#   3   lychee --offline --include-fragments     README.md CONTRIBUTING.md docs/
#   4   pytest docs/hooks · pytest scripts · node --test docs/javascripts/tests · node --test scripts/cdp
#   4b  docs/figures/tokens.py --css byte-equal to the huv-tokens block of docs/stylesheets/extra.css
#                                                ("skip (script absent)" until tokens.py lands)
#   5   docs/figures/sim/make_all.py --check     (both registries, captions, pages; tolerant until INT)
#   5b  docs/figures/check_figures.py --check    (standalone form; skipped when absent)
#   6   --examples: pytest tests/docs            (verify-examples + caption/toml/rosetta tests; the
#                                                compiled extension is copied from the mirror if absent)
#   7   --qa: serve the built site and run scripts/qa_site.mjs --sample in light/dark/print/phone
#                                                (page-form asserts warn unless HUV_QA_STRICT=1)
#   8   --figures: LOCAL ONLY — refused when CI is set — regenerate every generator on the figure
#       machine and `git diff --exit-code docs/assets/images` (reproducibility; DESIGN §9.3-7)
#   --strict exports HUV_FIGCHECK_STRICT=1 HUV_BADGELINT_STRICT=1 HUV_DOCS_STRICT_TOML=1 HUV_QA_STRICT=1
#            for this run (INT's final gate).
#
# Environment overrides (defaults = the scratch toolchain when it exists, else PATH):
#   DOCSVENV  the docs venv dir (mkdocs 1.6.1, material 9.7.7)   PROJPY  the project python (numpy, matplotlib)
#   FIGPY     python for --figures (the figure machine's)          LYCHEE  the lychee 0.24.2 binary
#   HUV_CLI   the CLI binary for test_rosetta.py                   CHROME  the Chrome binary for --qa
#   PORT      the http.server port for --qa (default 8793)         NODE    node (>= 22)
#   SITE_DIR  where to build the site (default: a fresh mktemp dir, printed at the end)
#   HUV_SCRATCH  the orchestrator's durable work dir (docsvenv/, mirror/, s0tools/ live under it)

WT="$1"; shift || true
if [ -z "$WT" ] || [ ! -d "$WT" ]; then echo "usage: scripts/docs_gates.sh <worktree> [--examples] [--qa] [--figures] [--strict]"; exit 99; fi
RUN_EXAMPLES=0; RUN_QA=0; RUN_FIGURES=0
for a in "$@"; do
  case "$a" in
    --examples) RUN_EXAMPLES=1;;
    --qa) RUN_QA=1;;
    --figures) RUN_FIGURES=1;;
    --strict) export HUV_FIGCHECK_STRICT=1 HUV_BADGELINT_STRICT=1 HUV_DOCS_STRICT_TOML=1 HUV_QA_STRICT=1;;
    *) echo "unknown flag: $a"; exit 99;;
  esac
done
cd "$WT" || { echo "no such worktree: $WT"; exit 99; }
WT=$(pwd -P)

SP="${HUV_SCRATCH:-/Users/user/.claude/projects/-Users-user-cdev-highuvlith/work}"
if [ -z "$DOCSVENV" ] && [ -x "$SP/docsvenv/bin/mkdocs" ]; then DOCSVENV="$SP/docsvenv"; fi
if [ -n "$DOCSVENV" ]; then DOCSPY="$DOCSVENV/bin/python"; MKDOCS="$DOCSVENV/bin/mkdocs"; else DOCSPY=python3; MKDOCS=mkdocs; fi
if [ -z "$PROJPY" ]; then
  if [ -x /Users/user/cdev/highuvlith/.venv/bin/python ]; then PROJPY=/Users/user/cdev/highuvlith/.venv/bin/python; else PROJPY=python3; fi
fi
if [ -z "$FIGPY" ]; then
  if [ -x /opt/homebrew/bin/python3 ]; then FIGPY=/opt/homebrew/bin/python3; else FIGPY="$PROJPY"; fi
fi
if [ -z "$LYCHEE" ]; then
  if [ -x "$SP/s0tools/lychee-aarch64-apple-darwin/lychee" ]; then LYCHEE="$SP/s0tools/lychee-aarch64-apple-darwin/lychee"; else LYCHEE=lychee; fi
fi
NODE="${NODE:-node}"
PORT="${PORT:-8793}"
export PYTHONDONTWRITEBYTECODE=1 HUV_DOCS_STRICT_NAV=1 NO_MKDOCS_2_WARNING=true
export HUV_CLI CHROME

fails=0
if [ -n "$SITE_DIR" ]; then SITE="$SITE_DIR"; mkdir -p "$SITE"; else SITE=$(mktemp -d /tmp/huv-site.XXXXXX); fi
gate_fail() { fails=$((fails+1)); }

echo "== 1 mkdocs build --strict (CI=true)"
if CI=true "$MKDOCS" build --strict -q -d "$SITE/highuvlith" >"$SITE/mkdocs.log" 2>&1; then echo "   ok"; else echo "   FAIL"; grep -v "^$" "$SITE/mkdocs.log" | grep -E "WARNING|ERROR|Error" | head -40; gate_fail; fi

echo "== 2 math lint (scripts/check_docs_math.py)"
out=$("$DOCSPY" scripts/check_docs_math.py docs README.md CONTRIBUTING.md 2>&1 | tail -1)
case "$out" in *" 0 finding"*) echo "   ok ($out)";; *) echo "   FAIL: $out"; "$DOCSPY" scripts/check_docs_math.py docs README.md CONTRIBUTING.md 2>&1 | head -30; gate_fail;; esac

echo "== 2b badge lint (scripts/check_docs_badges.py${HUV_BADGELINT_STRICT:+, strict})"
if [ -f scripts/check_docs_badges.py ]; then
  if "$DOCSPY" scripts/check_docs_badges.py docs/ README.md >"$SITE/badges.log" 2>&1; then echo "   ok ($(tail -1 "$SITE/badges.log"))"; else echo "   FAIL ($(tail -1 "$SITE/badges.log"))"; grep -iE "error" "$SITE/badges.log" | head -30; gate_fail; fi
else
  echo "   skip (script absent)"
fi

echo "== 2c Status lines (the CI grep)"
missing=0
for f in $(find docs \( -path docs/history -o -path docs/nodes -o -path docs/future -o -path docs/playground -o -path docs/includes -o -path docs/hooks -o -path docs/overrides -o -path docs/stylesheets -o -path docs/javascripts -o -path docs/assets -o -path docs/figures \) -prune -o -name '*.md' ! -name 'index.md' ! -name 'capability-matrix.md' -print); do
  grep -q '^\*\*Status:\*\*' "$f" || { echo "   missing **Status:** line: $f"; missing=1; }
done
if [ "$missing" = 0 ]; then echo "   ok"; else echo "   FAIL"; gate_fail; fi

echo "== 3 lychee --offline (README, CONTRIBUTING, docs)"
ly=$("$LYCHEE" --offline --no-progress --include-fragments README.md CONTRIBUTING.md docs 2>&1)
if echo "$ly" | grep -qE "0 Errors"; then echo "   ok ($(echo "$ly" | grep -oE '[0-9]+ Total'))"; else echo "   FAIL"; echo "$ly" | grep -E "✗|ERROR|Errors|\[" | head -40; gate_fail; fi

echo "== 4 hook, lint and JS tests"
g4=0
if "$DOCSPY" -m pytest -q docs/hooks -p no:cacheprovider >"$SITE/hooks.log" 2>&1; then echo "   hooks ok ($(tail -1 "$SITE/hooks.log"))"; else echo "   hooks FAIL"; tail -20 "$SITE/hooks.log"; g4=1; fi
if ls scripts/test_*.py >/dev/null 2>&1; then
  if "$DOCSPY" -m pytest -q scripts -p no:cacheprovider >"$SITE/scripts.log" 2>&1; then echo "   lint tests ok ($(tail -1 "$SITE/scripts.log"))"; else echo "   lint tests FAIL"; tail -20 "$SITE/scripts.log"; g4=1; fi
fi
if ls docs/javascripts/tests/*.test.mjs >/dev/null 2>&1; then
  if "$NODE" --test docs/javascripts/tests/*.test.mjs >"$SITE/js.log" 2>&1; then echo "   js ok ($(grep -E '^ℹ (pass|fail)' "$SITE/js.log" | tr '\n' ' '))"; else echo "   js FAIL"; tail -20 "$SITE/js.log"; g4=1; fi
fi
if ls scripts/cdp/*.test.mjs >/dev/null 2>&1; then
  if "$NODE" --test scripts/cdp/*.test.mjs >"$SITE/qa-js.log" 2>&1; then echo "   qa js ok ($(grep -E '^ℹ (pass|fail)' "$SITE/qa-js.log" | tr '\n' ' '))"; else echo "   qa js FAIL"; tail -20 "$SITE/qa-js.log"; g4=1; fi
fi
[ "$g4" = 1 ] && gate_fail

echo "== 4b tokens byte-equality (docs/figures/tokens.py --css vs extra.css)"
if [ -f docs/figures/tokens.py ]; then
  if ! grep -q 'huv-tokens:start' docs/stylesheets/extra.css; then echo "   FAIL (extra.css has no huv-tokens:start/end block)"; gate_fail
  elif "$PROJPY" docs/figures/tokens.py --css >"$SITE/tokens.css" 2>"$SITE/tokens.err" && sed -n '/huv-tokens:start/,/huv-tokens:end/p' docs/stylesheets/extra.css | diff -q - "$SITE/tokens.css" >/dev/null; then echo "   ok"
  else echo "   FAIL (tokens.py --css differs from the extra.css block)"; sed -n '/huv-tokens:start/,/huv-tokens:end/p' docs/stylesheets/extra.css | diff - "$SITE/tokens.css" | head -20; cat "$SITE/tokens.err" | head -5; gate_fail; fi
else
  echo "   skip (script absent)"
fi

echo "== 5 figure check (make_all.py --check${HUV_FIGCHECK_STRICT:+, strict})"
if PYTHONPATH="$WT/python" "$PROJPY" docs/figures/sim/make_all.py --check >"$SITE/figs.log" 2>&1; then echo "   ok ($(tail -1 "$SITE/figs.log"))"; else echo "   FAIL"; tail -25 "$SITE/figs.log"; gate_fail; fi

echo "== 5b figure registry check (check_figures.py --check)"
if [ -f docs/figures/check_figures.py ]; then
  if PYTHONPATH="$WT/python" "$PROJPY" docs/figures/check_figures.py --check >"$SITE/figs2.log" 2>&1; then echo "   ok ($(tail -1 "$SITE/figs2.log"))"; else echo "   FAIL"; tail -25 "$SITE/figs2.log"; gate_fail; fi
else
  echo "   skip (script absent)"
fi

if [ "$RUN_EXAMPLES" = 1 ]; then
  echo "== 6 docs examples (pytest tests/docs: verify-examples, caption/toml/rosetta tests)"
  if ! ls python/highuvlith/_native*.so >/dev/null 2>&1; then
    if cp "$SP"/mirror/python/highuvlith/_native*.so python/highuvlith/ 2>/dev/null; then echo "   (copied the compiled extension from the mirror)"; else echo "   (no compiled extension found: build it with maturin develop --release)"; fi
  fi
  if PYTHONPATH="$WT/python" MPLBACKEND=Agg "$PROJPY" -m pytest -q tests/docs -p no:cacheprovider >"$SITE/ex.log" 2>&1; then echo "   ok ($(tail -1 "$SITE/ex.log"))"; else echo "   FAIL"; grep -E "FAILED|Error|assert|drift|Timeout" "$SITE/ex.log" | head -30; tail -3 "$SITE/ex.log"; gate_fail; fi
fi

if [ "$RUN_QA" = 1 ]; then
  echo "== 7 headless QA (scripts/qa_site.mjs --sample${HUV_QA_STRICT:+, strict})"
  if [ ! -d "$SITE/highuvlith" ]; then echo "   FAIL (no built site)"; gate_fail
  else
    if "$DOCSPY" - "$PORT" <<'PY' >/dev/null 2>&1
import socket, sys
s = socket.socket(); s.settimeout(0.3)
sys.exit(0 if s.connect_ex(("127.0.0.1", int(sys.argv[1]))) != 0 else 1)
PY
    then :; else PORT=$("$DOCSPY" -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'); echo "   (port busy; using $PORT)"; fi
    ( cd "$SITE" && exec "$DOCSPY" -m http.server "$PORT" --bind 127.0.0.1 >"$SITE/http.log" 2>&1 ) &
    HTTP_PID=$!
    trap 'kill $HTTP_PID 2>/dev/null' EXIT
    for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
      if "$DOCSPY" -c "import urllib.request,sys; sys.exit(0 if urllib.request.urlopen('http://127.0.0.1:$PORT/highuvlith/').status==200 else 1)" >/dev/null 2>&1; then break; fi
      sleep 0.25
    done
    mkdir -p "$SITE/qa"
    if "$NODE" scripts/qa_site.mjs --site "http://127.0.0.1:$PORT/highuvlith" --modes light,dark,print,phone --sample --out "$SITE/qa" >"$SITE/qa.log" 2>&1; then echo "   ok ($(tail -1 "$SITE/qa.log"))"; else echo "   FAIL ($(tail -1 "$SITE/qa.log"))"; grep -E "^FAIL|^- \*\*" "$SITE/qa/qa-report.md" 2>/dev/null | head -30; tail -5 "$SITE/qa.log"; gate_fail; fi
    echo "   report: $SITE/qa/qa-report.md"
    kill $HTTP_PID 2>/dev/null; trap - EXIT
  fi
fi

if [ "$RUN_FIGURES" = 1 ]; then
  echo "== 8 figure reproducibility (--figures: regenerate on the figure machine, LOCAL ONLY)"
  if [ -n "$CI" ]; then echo "   FAIL: --figures is local-only (DESIGN §9.3-7, harness fact 5); refused because CI is set"; gate_fail
  else
    g8=0
    echo "   python: $FIGPY ($("$FIGPY" --version 2>&1))"
    if ! PYTHONPATH="$WT/python" "$FIGPY" docs/figures/sim/make_all.py >"$SITE/regen-sim.log" 2>&1; then echo "   make_all.py FAIL"; tail -15 "$SITE/regen-sim.log"; g8=1; fi
    for s in docs/figures/diagrams/make_spectrum_strip.py docs/figures/diagrams/make_pipeline_diagrams.py docs/figures/diagrams/make_physics_schematics.py docs/figures/diagrams/make_process_schematics.py docs/figures/diagrams/make_source_schematics.py \
             docs/figures/history/make_history_figures.py docs/figures/history/make_history_schematics.py docs/figures/history/make_history_animation.py \
             docs/figures/nodes/make_node_figures.py docs/figures/nodes/make_node_schematics.py docs/figures/future/make_future_figures.py docs/figures/future/make_future_schematics.py \
             docs/figures/reference/make_matrix_map.py docs/figures/gui/make_gui_media.py docs/figures/playground/make_playground_figures.py; do
      [ -f "$s" ] || continue
      if ! PYTHONPATH="$WT/python" "$FIGPY" "$s" >"$SITE/regen-$(basename "$s" .py).log" 2>&1; then echo "   $s FAIL"; tail -10 "$SITE/regen-$(basename "$s" .py).log"; g8=1; fi
    done
    if [ -f docs/figures/check_figures.py ]; then
      if ! PYTHONPATH="$WT/python" "$FIGPY" docs/figures/check_figures.py --platform-independent --regenerate >"$SITE/regen-pi.log" 2>&1; then echo "   check_figures.py --platform-independent --regenerate FAIL"; tail -10 "$SITE/regen-pi.log"; g8=1; fi
    fi
    if git diff --exit-code --stat -- docs/assets/images >"$SITE/regen-diff.log" 2>&1; then echo "   git diff docs/assets/images clean"; else echo "   git diff docs/assets/images NOT clean:"; head -30 "$SITE/regen-diff.log"; g8=1; fi
    [ "$g8" = 1 ] && gate_fail
    [ "$g8" = 0 ] && echo "   ok"
  fi
fi

echo "== site built at $SITE/highuvlith (serve the parent dir: cd $SITE && python3 -m http.server PORT --bind 127.0.0.1)"
echo "== RESULT: $fails failed gate(s)"
exit $fails
