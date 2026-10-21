#!/usr/bin/env bash
#
# install-deps-fedora.sh - install every build and test dependency for Kooka
# Pictura on Fedora 43/44, on x86_64 and aarch64 (including Fedora Asahi Remix
# on Apple Silicon, which is detected automatically).
#
# Covers the C++/Qt toolchain (gcc, clang + clangd, cmake, ninja, lld), Qt 6
# (Core/Gui/Widgets/Svg/Network/PrintSupport/Test, plus the private RHI headers
# CMakeLists.txt adds by hand), Little CMS 2, Vulkan + a Mesa software driver,
# the oracle tools (ImageMagick 7, psd-tools, Pillow, the system ICC profiles),
# Node/npm + the pinned OpenSpec CLI, the pinned Rust toolchain with
# rustfmt/clippy/llvm-tools/rust-analyzer, cargo-nextest/cargo-deny/sccache, and
# the Serena MCP server via uv.
#
# Usage: scripts/install-deps-fedora.sh [options]
#   --no-rust        skip rustup + cargo tooling
#   --no-node        skip node/npm + the OpenSpec CLI
#   --no-python      skip psd-tools + Pillow
#   --no-oracles     skip ImageMagick + the system ICC profiles
#   --no-fonts       skip the bundled UI fonts (Noto Sans, DejaVu)
#   --no-serena      skip uv + Serena
#   --packaging      also install the rpm/deb packaging tools
#   --force          proceed on an unsupported Fedora version or architecture
#   -h, --help       show this help
#
# Every step is idempotent, so re-running is safe.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

with_rust=1
with_node=1
with_python=1
with_oracles=1
with_fonts=1
with_serena=1
with_packaging=0
force=0

while [ $# -gt 0 ]; do
    case "$1" in
        --no-rust) with_rust=0 ;;
        --no-node) with_node=0 ;;
        --no-python) with_python=0 ;;
        --no-oracles) with_oracles=0 ;;
        --no-fonts) with_fonts=0 ;;
        --no-serena) with_serena=0 ;;
        --packaging) with_packaging=1 ;;
        --force) force=1 ;;
        -h | --help)
            awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"
            exit 0
            ;;
        *)
            echo "install-deps-fedora: unknown option: $1" >&2
            exit 2
            ;;
    esac
    shift
done

# --- platform check -----------------------------------------------------------
arch="$(uname -m)"
case "$arch" in
    x86_64 | aarch64) ;;
    *)
        echo "install-deps-fedora: unsupported architecture: $arch (x86_64/aarch64)" >&2
        [ "$force" -eq 1 ] || exit 1
        ;;
esac

# Fedora Asahi Remix reports ID=fedora-asahi-remix; detect it via os-release or
# the kernel name so we can pick the right Vulkan driver story and accept its release
# cadence (Asahi Remix trails mainline Fedora by a release or so).
asahi=0
if [ -r /etc/os-release ]; then
    . /etc/os-release
    # Asahi Remix sets ID=fedora-asahi-remix (ID_LIKE=fedora), not ID=fedora.
    case "${ID:-}:${VARIANT_ID:-}:${VARIANT:-}:$(uname -r)" in
        *[Aa]sahi*) asahi=1 ;;
    esac
    if [ "${ID:-}" != "fedora" ] && [ "$asahi" -ne 1 ]; then
        echo "install-deps-fedora: this script targets Fedora, found ID=${ID:-unknown}" >&2
        [ "$force" -eq 1 ] || exit 1
    fi
    if [ "$asahi" -eq 1 ]; then
        supported="42 | 43 | 44"
    else
        supported="43 | 44"
    fi
    case "${VERSION_ID:-}" in
        43 | 44) ;;
        42)
            if [ "$asahi" -ne 1 ]; then
                echo "install-deps-fedora: tested on Fedora ${supported// | //}, found 42" >&2
                [ "$force" -eq 1 ] || exit 1
            fi
            ;;
        *)
            echo "install-deps-fedora: tested on Fedora ${supported// | //}, found ${VERSION_ID:-unknown}" >&2
            [ "$force" -eq 1 ] || exit 1
            ;;
    esac
fi
platform_label="$arch"
[ "$asahi" -eq 1 ] && platform_label="$arch, Asahi"

if [ "$(id -u)" -eq 0 ]; then
    SUDO=""
elif command -v sudo >/dev/null 2>&1; then
    SUDO="sudo"
else
    echo "install-deps-fedora: need root or sudo to install packages" >&2
    exit 1
fi

dnf_install() {
    echo "== dnf install: $*"
    $SUDO dnf install -y "$@"
}

# --- package groups -----------------------------------------------------------
# gcc for the C++ shell; clang + clang-tools-extra give clangd (Serena's C++
# language server); lld is the fast linker Cargo expects, and CMake prefers mold
# when the compiler accepts it.
base_pkgs=(
    gcc-c++ make cmake ninja-build lld
    clang clang-tools-extra clang-devel
    pkgconf-pkg-config git curl
)

# qt6-qtbase-devel ships qmake6 and Qt6Test; qt6-qtbase-private-devel ships the
# versioned QtGui/rhi headers CMakeLists.txt adds explicitly.
qt_pkgs=(
    qt6-qtbase qt6-qtbase-gui qt6-qtbase-devel qt6-qtbase-private-devel
    qt6-qtsvg qt6-qtsvg-devel
)

# lcms2: the C++ binary links -llcms2. Vulkan loader + headers for the wgpu GPU
# path and --interop-probe; mesa-vulkan-drivers is the software (lavapipe)
# adapter headless GPU tests fall back to. On Fedora Asahi Remix the same
# package also carries the Honeykrisp driver for the Apple GPU, so no extra
# package is needed there (the Asahi Mesa fork is pulled in by the distro).
graphics_pkgs=(
    lcms2 lcms2-devel
    vulkan-loader vulkan-loader-devel vulkan-headers vulkan-tools
    mesa-vulkan-drivers mesa-dri-drivers
)

# ImageMagick 7 provides `magick`; colord ships the sRGB/AdobeRGB ICC profiles
# the pictura-color oracle reads from /usr/share/color/icc/colord.
oracle_pkgs=(ImageMagick colord)

font_pkgs=(google-noto-sans-fonts dejavu-sans-fonts)

# npm is a separate package from nodejs on Fedora.
node_pkgs=(nodejs nodejs-npm)

# pip for the psd-tools install; uv for Serena.
python_pkgs=(python3 python3-pip)
serena_pkgs=(uv)

# Only needed to build the .rpm/.deb locally (packaging/build-packages.sh).
packaging_pkgs=(rpm-build rpmdevtools dpkg fuse-libs)

pkgs=("${base_pkgs[@]}" "${qt_pkgs[@]}" "${graphics_pkgs[@]}")
[ "$with_oracles" -eq 1 ] && pkgs+=("${oracle_pkgs[@]}")
[ "$with_fonts" -eq 1 ] && pkgs+=("${font_pkgs[@]}")
[ "$with_node" -eq 1 ] && pkgs+=("${node_pkgs[@]}")
[ "$with_python" -eq 1 ] && pkgs+=("${python_pkgs[@]}")
[ "$with_serena" -eq 1 ] && pkgs+=("${serena_pkgs[@]}")
[ "$with_packaging" -eq 1 ] && pkgs+=("${packaging_pkgs[@]}")

dnf_install "${pkgs[@]}"

# --- Rust toolchain (pinned by rust-toolchain.toml) ---------------------------
if [ "$with_rust" -eq 1 ]; then
    export PATH="$HOME/.cargo/bin:$PATH"

    if ! command -v rustup >/dev/null 2>&1; then
        if command -v rustup-init >/dev/null 2>&1; then
            echo "== rustup-init"
            rustup-init -y --no-modify-path --default-toolchain none
            export PATH="$HOME/.cargo/bin:$PATH"
        else
            echo "== install rustup (official installer)"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
                sh -s -- -y --no-modify-path --default-toolchain none
            export PATH="$HOME/.cargo/bin:$PATH"
        fi
    fi

    channel="$(sed -n 's/^channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' rust-toolchain.toml | head -1)"
    channel="${channel:-stable}"

    echo "== rustup toolchain install $channel"
    rustup toolchain install "$channel" --profile default
    rustup component add --toolchain "$channel" rust-analyzer || true
    rustup default "$channel"

    # sccache and cargo-deny are packaged on Fedora; nextest is not.
    dnf_install sccache cargo-deny

    if ! command -v cargo-nextest >/dev/null 2>&1; then
        echo "== cargo install cargo-nextest (builds from source; slow on aarch64)"
        cargo install cargo-nextest --locked
    fi
fi

# --- Python oracle tooling ----------------------------------------------------
# psd-tools is not packaged; install it user-local. Pillow is imported directly
# by scripts/generate-fixtures.py.
if [ "$with_python" -eq 1 ]; then
    echo "== pip install psd-tools + Pillow"
    python3 -m pip install --user --break-system-packages --upgrade \
        "psd-tools>=1.19" Pillow
fi

# --- Node + OpenSpec ----------------------------------------------------------
if [ "$with_node" -eq 1 ]; then
    echo "== npm install -g @fission-ai/openspec@1.13.2"
    $SUDO npm install -g @fission-ai/openspec@1.13.2
fi

# --- Serena (MCP server, on PATH for opencode.json) ---------------------------
if [ "$with_serena" -eq 1 ]; then
    echo "== uv tool install serena-agent"
    uv tool install -p 3.13 serena-agent
fi

# --- summary ------------------------------------------------------------------
echo
echo "install-deps-fedora: done (Fedora ${VERSION_ID:-?}, $platform_label)"
echo
echo "Next steps:"
echo "  cmake -S . -B build -G Ninja"
echo "  cmake --build build --parallel"
echo "  bash scripts/verify-fast.sh"
echo "  ./build/pictura --headless --self-test"
if [ "$asahi" -eq 1 ]; then
    echo
    echo "Asahi: 'vulkaninfo --summary' should list the Apple GPU (Honeykrisp);"
    echo "  if headless GPU tests misbehave, force lavapipe with:"
    echo "  export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.aarch64.json"
fi
if [ "$with_rust" -eq 1 ]; then
    echo
    echo "Optional build cache: export RUSTC_WRAPPER=sccache"
fi
if [ "$with_serena" -eq 1 ]; then
    echo
    echo "Serena installs its shim under ~/.local/bin; add it to PATH, then:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo "  serena project index   # once, after a fresh clone"
fi
