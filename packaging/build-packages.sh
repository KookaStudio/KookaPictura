#!/usr/bin/env bash
# Repackage a linuxdeploy AppDir (.github/actions/linux-bundle) as a .deb and an
# .rpm. Qt and the other bundled libraries live privately under
# /opt/kooka-pictura (the binary's $ORIGIN rpath and qt.conf already point
# there); /usr/bin/pictura is a symlink and the desktop entry and icon go to
# /usr/share. Only host libraries the bundle does not carry become package
# dependencies.
#
# usage: packaging/build-packages.sh VERSION APPDIR OUTDIR [deb|rpm]...
# Needs dpkg-deb + dpkg-shlibdeps for deb, rpmbuild for rpm.
set -euo pipefail

version=${1#v}
appdir=$(realpath "$2")
outdir=$(realpath "$3")
shift 3
[ $# -gt 0 ] || set -- deb rpm

name=kooka-pictura
prefix=/opt/$name
summary="Raster image editor modeled on Photoshop CS6"
description="Kooka Pictura is a documentation-first reimplementation of Adobe
Photoshop CS6 in Rust and Qt 6. This package bundles its own Qt under $prefix."
homepage=https://github.com/KookaStudio/KookaPictura
maintainer="Kooka Studio <kookastudio@users.noreply.github.com>"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
root=$work/root

mkdir -p "$root$prefix" "$root/usr/bin" "$root/usr/share"
for dir in "$appdir"/usr/*; do
    [ "$(basename "$dir")" = share ] || cp -a "$dir" "$root$prefix/"
done
for dir in applications icons metainfo; do
    if [ -d "$appdir/usr/share/$dir" ]; then
        cp -a "$appdir/usr/share/$dir" "$root/usr/share/"
    fi
done
install -d "$root/usr/share/doc/$name"
install -m644 LICENSE "$root/usr/share/doc/$name/copyright"
install -m644 THIRD-PARTY-LICENSES "$root/usr/share/doc/$name/THIRD-PARTY-LICENSES"
# Anything else linuxdeploy put in share/ (bundled libraries' copyright files)
# stays private so it cannot collide with distro packages.
for dir in "$appdir"/usr/share/*; do
    case $(basename "$dir") in
        applications | icons | metainfo) ;;
        *) mkdir -p "$root$prefix/share" && cp -a "$dir" "$root$prefix/share/" ;;
    esac
done
ln -s "../..$prefix/bin/pictura" "$root/usr/bin/pictura"

mapfile -t elves < <(find "$root$prefix" -type f -exec sh -c \
    'head -c4 "$1" | grep -q "^.ELF" && echo "$1"' _ {} \;)

build_deb() {
    local pkg=$work/deb
    cp -a "$root" "$pkg"
    mkdir -p "$pkg/DEBIAN" "$work/shlibs/debian"
    # dpkg-shlibdeps wants a source package around it; the stub only names one.
    printf 'Source: %s\n\nPackage: %s\nArchitecture: amd64\n' "$name" "$name" \
        >"$work/shlibs/debian/control"
    local depends
    depends=$(cd "$work/shlibs" && dpkg-shlibdeps -O --ignore-missing-info \
        -l"$pkg$prefix/lib" "${elves[@]/#$root/$pkg}" |
        sed -n 's/^shlibs:Depends=//p')
    cat >"$pkg/DEBIAN/control" <<EOF
Package: $name
Version: $version
Architecture: amd64
Maintainer: $maintainer
Installed-Size: $(du -sk --exclude=DEBIAN "$pkg" | cut -f1)
Depends: $depends
Section: graphics
Priority: optional
Homepage: $homepage
Description: $summary
$(sed 's/^/ /' <<<"$description")
EOF
    dpkg-deb --root-owner-group -Zxz --build "$pkg" "$outdir/KookaPictura_${version}_amd64.deb"
}

build_rpm() {
    # The bundled sonames must not leak into Provides, and must not be Required
    # from the host either; everything else rpm's ELF scan finds is a real dep.
    local bundled
    bundled=$(find "$root$prefix/lib" -name '*.so*' -printf '%f\n' | sort -u |
        sed 's/[.+]/\\&/g' | paste -sd'|')
    mkdir -p "$work/rpm"
    cat >"$work/rpm/$name.spec" <<EOF
%global __provides_exclude_from ^$prefix/
%global __requires_exclude ^($bundled)
%global debug_package %{nil}
%global __os_install_post %{nil}
%global _build_id_links none

Name: $name
Version: $version
Release: 1
Summary: $summary
License: GPL-3.0-or-later
URL: $homepage
ExclusiveArch: x86_64

%description
$description

%install
cp -a "$root"/. %{buildroot}/

%files
$prefix
/usr/bin/pictura
/usr/share/applications/pictura.desktop
/usr/share/icons/hicolor/512x512/apps/pictura.png
/usr/share/metainfo/pictura.metainfo.xml
%license /usr/share/doc/$name/copyright
%license /usr/share/doc/$name/THIRD-PARTY-LICENSES
EOF
    rpmbuild -bb --define "_topdir $work/rpm" "$work/rpm/$name.spec"
    cp "$work"/rpm/RPMS/x86_64/*.rpm "$outdir/KookaPictura-${version}-1.x86_64.rpm"
}

for format in "$@"; do
    "build_$format"
done
