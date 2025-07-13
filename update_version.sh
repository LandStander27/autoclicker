#!/bin/sh

version=$( set -o pipefail; git describe --long --abbrev=7 2>/dev/null | sed 's/\([^-]*-g\)/r\1/;s/-/./g' || printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short=7 HEAD)")
echo new version: $version

sed -i "s/version: \&str = .*;/version: \&str = \"$version\";/" version/src/lib.rs