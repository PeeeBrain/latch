#!/usr/bin/env bash
# verify-latch helper. Run from anywhere inside the repo.
#   verify.sh doctor                     is this checkout worth driving?
#   verify.sh drive <script> [WxH]       drive the working tree, save screenshots
#   verify.sh before <script> [rev] [WxH] same script on rev (default: merge-base
#                                        with main) and on the working tree
# VAULT=/path/vault.enc reuses a sandbox Vault across runs; otherwise each run
# gets a fresh one inside its evidence directory.
set -uo pipefail
root=$(git rev-parse --show-toplevel) || exit 1
cd "$root"
driver=crates/latch-desktop/src/app/drive.rs

# drive_in <checkout> <script> <out dir> <size> <target dir>
# Each checkout needs its own target dir: with a shared one, Cargo's fingerprint
# still names the other checkout's sources and silently reuses its binary.
drive_in() {
  mkdir -p "$3"
  (cd "$1" && LATCH_DRIVE_SCRIPT=$2 LATCH_DRIVE_OUT=$3 LATCH_DRIVE_SIZE=$4 \
    LATCH_DRIVE_VAULT=${VAULT:-$3/vault/vault.enc} CARGO_TARGET_DIR=$5 \
    cargo test -p latch-desktop --locked --quiet app::drive::drive -- --ignored --exact --nocapture) \
    2>&1 | grep -v '^running 1 test$' | tee "$3/transcript.log"
  return "${PIPESTATUS[0]}"
}

case "${1:-}" in
doctor)
  echo "rev:       $(git rev-parse --short HEAD) $(git diff --quiet HEAD -- crates && echo clean || echo 'dirty crates/')"
  echo "toolchain: $(rustc --version)"
  [ -f "$driver" ] || { echo "driver:    MISSING $driver"; exit 1; }
  cargo check -p latch-desktop --all-targets --locked --quiet && echo "check:     ok" || { echo "check:     FAILED"; exit 1; }
  ;;
drive | before)
  script=$(realpath "${2:?usage: verify.sh $1 <script>}") || exit 1
  name=$(basename "$script" .txt)
  run=$root/target/verify/$name/$(date -u +%Y%m%dT%H%M%SZ)
  mkdir -p "$run" && cp "$script" "$run/script.txt"
  if [ "$1" = drive ]; then
    drive_in "$root" "$script" "$run" "${3:-720x480}" "$root/target"; code=$?
  else
    rev=$(git rev-parse --short "${3:-$(git merge-base HEAD main)}") || exit 1
    tree=$root/target/verify/worktrees/$rev
    [ -d "$tree" ] || git worktree add --detach "$tree" "$rev" >/dev/null || exit 1
    # The current driver drives the old app, so both sides read the script the same way.
    cp "$driver" "$tree/$driver"
    grep -q '^mod drive;$' "$tree/crates/latch-desktop/src/app.rs" ||
      printf '\n#[cfg(test)]\nmod drive;\n' >> "$tree/crates/latch-desktop/src/app.rs"
    echo "== before ($rev)"
    VAULT=${VAULT:+$VAULT.before} drive_in "$tree" "$script" "$run/before" "${4:-720x480}" "$root/target/verify/base-target"; before=$?
    echo "== after (working tree)"
    drive_in "$root" "$script" "$run/after" "${4:-720x480}" "$root/target"; code=$?
    if command -v magick >/dev/null; then
      for shot in "$run"/after/*.png; do
        base=$run/before/$(basename "$shot")
        [ -f "$base" ] || continue
        magick "$base" "$shot" +append "$run/$(basename "$shot" .png)-before-after.png"
        # Rendering is deterministic: 0 means the change did not touch this screen.
        echo "pixels changed in $(basename "$shot"): $(magick compare -metric AE "$base" "$shot" null: 2>&1 | cut -d' ' -f1)"
      done
    fi
    [ "$before" = 0 ] || echo "note: the script failed on $rev; see $run/before/transcript.log"
  fi
  echo "evidence: $run"
  find "$run" -maxdepth 2 -name '*.png' | sort | sed 's/^/  /'
  exit "$code"
  ;;
*) sed -n '2,8p' "$0"; exit 2 ;;
esac
