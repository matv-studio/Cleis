#!/bin/bash
# Builds the Cleis Server QPKG for QNAP (D-78) with QNAP's QDK (GPL build tool, not shipped in
# the repository; its installer script qinstall.sh goes into the package).
#   packaging/qnap/build.sh VERSION x86_64=PATH/cleis-server [arm_64=PATH/cleis-server]
# Needs `qbuild` on PATH (CI: the qdk .deb from github.com/qnap-dev/QDK). Output: dist/*.qpkg.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
ver=$1; shift
work=$(mktemp -d)
cp -r "$here/." "$work/"
rm -f "$work/build.sh"
sed -i "s/^QPKG_VER=.*/QPKG_VER=\"$ver\"/" "$work/qpkg.cfg"
cp "$root/THIRD_PARTY.md" "$work/shared/"
for a in "$@"; do
  arch=${a%%=*}; bin=${a#*=}
  mkdir -p "$work/$arch"
  install -m 0755 "$bin" "$work/$arch/cleis-server"
done
(cd "$work" && qbuild --build-dir build)
mkdir -p "$root/dist"
cp "$work"/build/*.qpkg "$root/dist/"
ls -la "$root/dist/"*.qpkg
