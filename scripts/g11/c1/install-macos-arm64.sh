#!/bin/bash
# Isolated reproduction recipe, not a Belenios implementation. Requires Apple CLT, curl, git, make and Node.
set -euo pipefail
case "$(uname -s)/$(uname -m)" in Darwin/arm64) ;; *) echo 'This recipe is pinned for macOS arm64 only.' >&2; exit 2;; esac
test "$#" -eq 1 || { echo 'Usage: bash install-macos-arm64.sh /ABSOLUTE/NEW/CACHE' >&2; exit 2; }
CACHE="$1"
case "$CACHE" in /*) ;; *) echo 'Use a new absolute cache path.' >&2; exit 2;; esac
test ! -e "$CACHE" || { echo 'Refusing to overwrite an existing cache.' >&2; exit 2; }
REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
umask 077
mkdir "$CACHE"
cd "$CACHE"
fetch() {
  local file="$1" url="$2" digest="$3"
  curl --http1.1 -fLsS --retry 2 --retry-all-errors --connect-timeout 30 --max-time 1200 "$url" -o "$file"
  local actual
  actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  test "$actual" = "$digest" || { echo "SHA256 mismatch: $file" >&2; exit 3; }
}
fetch opam https://github.com/ocaml/opam/releases/download/2.6.1/opam-2.6.1-arm64-macos 3e7f67bda3ba0bf72f597917b17dc337ed5a1bd48224df8b747bb35ce05c569e
chmod u+x opam
fetch opam-repository.tar.gz https://codeload.github.com/ocaml/opam-repository/tar.gz/4093222dece7c1c8f93dc2fd8dd7eec15fa80fed 0f8736b8977793226f48b62e242ece75b09c632b280184eaf08ea44846ac2185
fetch belenios-3.0.tar.gz https://www.belenios.org/releases/belenios-3.0.tar.gz 6e9fed14ccf849632b84570cbe852bea3cbe30b5c9baf6a4a3cad7e7d16c62f9
fetch gmp.tar.xz https://ftp.gnu.org/gnu/gmp/gmp-6.3.0.tar.xz a3c2b80201b89e68616f4ad30bc66aee4927c3ce50e33929ca819d5c43538898
fetch sodium.tar.gz https://download.libsodium.org/libsodium/releases/libsodium-1.0.20.tar.gz ebb65ef6ca439333c2bb41a0c1990587288da07f6c7fd07cb3a18cc18d30ce19
fetch pkgconf.tar.xz https://distfiles.dereferenced.org/pkgconf/pkgconf-2.3.0.tar.xz 3a9080ac51d03615e7c1910a0a2a8df08424892b5f13b0628a204d3fcce0ea8b
fetch zlib.tar.gz https://zlib.net/fossils/zlib-1.3.1.tar.gz 9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23
fetch base.tar.gz https://opam.ocaml.org/cache/md5/04/04572fc23a4651604cfcab83f720cb4c 9bf9e503e2bd010325c7b0f5ae007ce1e5d3da61f0e06d2fbb64e9d12ccff93c
tar -xzf opam-repository.tar.gz
tar -xJf gmp.tar.xz
tar -xzf sodium.tar.gz
tar -xJf pkgconf.tar.xz
tar -xzf zlib.tar.gz
tar -xzf base.tar.gz
for package in gmp-6.3.0 libsodium-1.0.20 pkgconf-2.3.0; do
  (cd "$package" && ./configure --prefix="$CACHE/native" --disable-shared && make -j2 && make install) > "$package.build.log" 2>&1
done
(cd zlib-1.3.1 && ./configure --prefix="$CACHE/native" --static && make -j2 && make install) > zlib.build.log 2>&1
ln -s pkgconf "$CACHE/native/bin/pkg-config"
export PATH="$CACHE/native/bin:$PATH"
export C_INCLUDE_PATH="$CACHE/native/include"
export LIBRARY_PATH="$CACHE/native/lib"
export CFLAGS="-I$CACHE/native/include"
export LDFLAGS="-L$CACHE/native/lib"
export PKG_CONFIG_PATH="$CACHE/native/lib/pkgconfig"
export XDG_CACHE_HOME="$CACHE/cache"
export OPAMNODEPEXTS=true
opam_local() { local subcommand="$1"; shift; "$CACHE/opam" "$subcommand" --root="$CACHE/opam-root" "$@"; }
opam_local init --bare --no-setup --yes --kind=local belenios-20240913 "$CACHE/opam-repository-4093222dece7c1c8f93dc2fd8dd7eec15fa80fed" > opam-init.log 2>&1
opam_local option --global 'archive-mirrors=["https://opam.ocaml.org/cache"]'
opam_local switch create 4.14.2 ocaml-base-compiler.4.14.2 --jobs=2 --yes --no-depexts > opam-compiler.log 2>&1
(cd base-0.16.3 && patch -p1 < "$REPO_ROOT/scripts/g11/c1/base-arm64-clang17.patch")
opam_local pin add base.v0.16.3 "$CACHE/base-0.16.3" --switch=4.14.2 --no-action --yes > base-pin.log 2>&1
opam_local install dune atdgen zarith cryptokit cmdliner yojson hex cohttp-lwt-unix --switch=4.14.2 --jobs=2 --yes --no-depexts > opam-dependencies.log 2>&1
git clone --depth 1 --branch 3.0 https://github.com/glondu/belenios.git belenios-git > source-clone.log 2>&1
test "$(git -C belenios-git rev-parse HEAD)" = 5a1596cefcc0944467ddc4bc9cdb0f24013b0d05
(cd belenios-git && opam_local exec --switch=4.14.2 -- dune build --profile release -j2 -p belenios-platform,belenios-platform-native,belenios-lib,belenios-tool) > belenios-build.log 2>&1
opam_local list --switch=4.14.2 --installed --columns=name,version --short > dependencies.txt
opam_local switch export --switch=4.14.2 dependencies.export
"$CACHE/belenios-git/_build/install/default/bin/belenios-tool" --version
echo "Belenios CLI: $CACHE/belenios-git/_build/install/default/bin/belenios-tool"
# Private credentials and secret shares must remain outside the VeriVoting repository.
