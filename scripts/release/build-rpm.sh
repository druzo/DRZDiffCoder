# scripts/release/build-rpm.sh
# Build .rpm packages for the current Linux target via rpmbuild + drz-app.spec.
# Outputs go to releases/${VERSION}/${folder}/drzdiff-${VERSION}-1.${rpm_arch}.rpm
#
# Usage:  build-rpm.sh <x86_64|arm64> [<stage-dir>]
# If <stage-dir> is omitted, uses releases/${VERSION}/linux-<arch>/.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
VERSION="${VERSION:?missing VERSION}"
ARCH="${1:?usage: $0 <x86_64|arm64> [<stage-dir>]}"
STAGE="${2:-${REPO_ROOT}/releases/${VERSION}/linux-${ARCH}}"

case "$ARCH" in
  x86_64) RPM_ARCH="x86_64"; FOLDER="linux-x86_64" ;;
  arm64)  RPM_ARCH="aarch64"; FOLDER="linux-arm64" ;;
  *) echo "unknown arch $ARCH" >&2; exit 1 ;;
esac

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "rpmbuild not found — apt install rpm" >&2
  exit 1
fi

[ -f "$STAGE/drzdiff" ] || {
  echo "missing $STAGE/drzdiff — run build-linux.sh first" >&2
  exit 1
}

# rpmbuild refuses to run as root unless --force is passed.
# In CI we run as a normal user; locally, fall back to --force.
RPM_FORCE=""
if [ "$(id -u)" -eq 0 ]; then
  RPM_FORCE="--force"
fi

RPM_TOP="$(mktemp -d -t drzdiff-rpmbuild.XXXXXX)"
trap 'rm -rf "$RPM_TOP"' EXIT

mkdir -p "$RPM_TOP"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

# Stage the payload + spec --------------------------------------------------
SPEC="$RPM_TOP/SPECS/drz-app.spec"
sed -e "s|@VERSION@|${VERSION}|g" \
    -e "s|@RELEASE@|1|g" \
    -e "s|@ARCH@|${RPM_ARCH}|g" \
    -e "s|@PAYLOAD@|${STAGE}/drzdiff|g" \
    -e "s|@ICON@|${REPO_ROOT}/icons/AppIcon.png|g" \
    -e "s|@LICENSE@|${REPO_ROOT}/LICENSE|g" \
    "$REPO_ROOT/scripts/release/drz-app.spec" > "$SPEC"

# rpmbuild requires a writable home for ~/.rpmdb.
export HOME="${HOME:-/root}"

cd "$REPO_ROOT"
fakeroot rpmbuild -bb $RPM_FORCE \
  --define "_topdir $RPM_TOP" \
  --define "_rpmdir $STAGE" \
  --define "_rpmfilename drzdiff-${VERSION}-1.${RPM_ARCH}.rpm" \
  "$SPEC" 2>&1 | tail -15

# Verify output
RPM_OUT="$STAGE/drzdiff-${VERSION}-1.${RPM_ARCH}.rpm"
if [ ! -f "$RPM_OUT" ]; then
  # rpmbuild may have written under _rpmdir/<arch> subdir
  RPM_OUT="$(find "$STAGE" -maxdepth 3 -name "drzdiff-*-1.${RPM_ARCH}.rpm" -type f | head -1)"
fi
[ -n "$RPM_OUT" ] && [ -f "$RPM_OUT" ] || {
  echo "rpmbuild succeeded but no .rpm found in $STAGE" >&2
  exit 1
}

echo "[rpm] $(basename "$RPM_OUT") OK"