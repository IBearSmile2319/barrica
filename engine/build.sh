#!/bin/sh
# Compila el motor de Barrica a partir del código libre de CrossOver (licencia de CrossOver, sección 2).
# Lado Unix x86_64 (corre bajo Rosetta) y lado Windows i386 + x86_64 (WoW64 nuevo). Ver docs/MOTOR.md.
#
#   engine/build.sh          todo; lo ya hecho se salta
#   engine/build.sh wine     solo recompila Wine (tras tocar sus fuentes)
#
# Requisitos: Xcode, Rosetta y `brew install bison mingw-w64 autoconf automake libtool cmake`.
# Resultado: engine/dist (bin/wine, lib/…); bundle.sh lo mete en Barrica.app.
set -eu

VERSION=26.3.0
SHA256=ac99c8ca4b3848f3e81784135f023df266b61c2345726ea55a50b3e030dd6872
MONO=10.4.1    # lo que pide dlls/appwiz.cpl/addons.c de Wine 11.0
GECKO=2.47.4
# El nettle del tarball viene sin Makefile.in. El oficial de GNU tiene los mismos 510 .c/.h, byte a byte.
NETTLE=3.10
NETTLE_SHA256=b4c518adb174e484cb4acea54118f02380c7133771e7e9beb98a0787194ee47c
# MoltenVK oficial (Vulkan → Metal, Apache-2.0): el win32u de CodeWeavers lo exige en macOS. Más nuevo que
# el 1.2.10 de CrossOver 26.3; Wine solo usa sus extensiones públicas (VK_MVK_macos_surface).
MOLTENVK=v1.4.2
MOLTENVK_SHA256=f95765a6229cb7b915990a2890ce12ebe36a730b021545d3d52ae69ce4c4024e
# DXMT (DirectX 10/11 → Metal): la 0.80 es la última con licencia MIT. Sin checksum oficial: fijado al bajarla.
DXMT=v0.80
DXMT_SHA256=8f260e36b5739e68f3bad613381441385c4dc7b85b78ba8de653d5a6a264529d

HERE=$(cd "$(dirname "$0")" && pwd)
WORK=${BARRICA_ENGINE_WORK:-$HOME/Library/Caches/Barrica/engine} # sin espacios: autotools no los soporta
SRC=$WORK/src
DEPS=$WORK/deps # prefijo x86_64 propio: nunca se mezcla con las librerías arm64 de Homebrew
DIST=$HERE/dist
JOBS=$(sysctl -n hw.ncpu)
BREW=$(brew --prefix)

# Compilador nativo arm64 que genera x86_64; --build=x86_64 hace que configure ejecute sus pruebas bajo Rosetta.
export MACOSX_DEPLOYMENT_TARGET=10.15 # el mismo mínimo que CrossOver 26.3
export CC="clang -arch x86_64" CXX="clang++ -arch x86_64" OBJC="clang -arch x86_64"
export CFLAGS="-O2" CXXFLAGS="-O2" OBJCFLAGS="-O2" CROSSCFLAGS="-O2"
export CPPFLAGS="-I$DEPS/include" LDFLAGS="-L$DEPS/lib"
export PKG_CONFIG_LIBDIR=$DEPS/lib/pkgconfig
export PATH="$BREW/opt/bison/bin:$BREW/bin:$PATH"
X86=--build=x86_64-apple-darwin

step() { printf '\n==> %s\n' "$*"; }
# Ojo: `set -e` no salta dentro de una lista `a && b` salvo en el último comando. Un paso por línea.

fetch() {
    [ -d "$SRC/wine" ] && return
    step "Fuentes de CrossOver $VERSION"
    mkdir -p "$WORK"
    cd "$WORK"
    tarball=crossover-sources-$VERSION.tar.gz
    [ -f "$tarball" ] || curl -fL -o "$tarball" "https://media.codeweavers.com/pub/crossover/source/$tarball"
    echo "$SHA256  $tarball" | shasum -a 256 -c -
    tar -xzf "$tarball"
    mv sources src
}

# Parches propios (engine/patches/*.patch, -p1 desde src/). Los ya aplicados se saltan.
patches() {
    for p in "$HERE"/patches/*.patch; do
        [ -f "$p" ] || continue
        if patch -d "$SRC" -p1 -R -s -f --dry-run < "$p" > /dev/null; then continue; fi
        step "Parche $(basename "$p")"
        patch -d "$SRC" -p1 -s < "$p"
    done
}

# gnutls (schannel: HTTPS en launchers y juegos) y freetype (fuentes). gmp y nettle van estáticos
# dentro de gnutls, así el motor solo lleva dos dylibs. Todo sale del tarball de CodeWeavers.
deps() {
    mkdir -p "$DEPS/lib"
    if [ ! -f "$DEPS/lib/libgmp.a" ]; then
        step "gmp"
        cd "$SRC/gnutls/gmp"
        ./configure $X86 --prefix="$DEPS" --disable-shared --with-pic --disable-assembly
        make -j"$JOBS"
        make install
    fi
    if [ ! -f "$DEPS/lib/libhogweed.a" ]; then
        step "nettle $NETTLE"
        mkdir -p "$WORK/addons"
        cd "$WORK/addons"
        [ -f "nettle-$NETTLE.tar.gz" ] || curl -fL -O "https://ftp.gnu.org/gnu/nettle/nettle-$NETTLE.tar.gz"
        echo "$NETTLE_SHA256  nettle-$NETTLE.tar.gz" | shasum -a 256 -c -
        rm -rf "nettle-$NETTLE"
        tar -xzf "nettle-$NETTLE.tar.gz"
        cd "nettle-$NETTLE"
        ./configure $X86 --prefix="$DEPS" --libdir="$DEPS/lib" --disable-shared --enable-pic --disable-documentation --disable-openssl
        make -j"$JOBS"
        make install
    fi
    if [ ! -f "$DEPS/lib/libgnutls.30.dylib" ]; then
        step "gnutls"
        cd "$SRC/gnutls/gnutls"
        ./configure $X86 --prefix="$DEPS" --disable-static --disable-doc --disable-tools --disable-cxx --disable-tests \
            --with-included-libtasn1 --with-included-unistring --without-p11-kit --without-idn \
            --without-brotli --without-zstd --without-tpm --without-tpm2 \
            NETTLE_CFLAGS="-I$DEPS/include" NETTLE_LIBS="$DEPS/lib/libnettle.a" \
            HOGWEED_CFLAGS="-I$DEPS/include" HOGWEED_LIBS="$DEPS/lib/libhogweed.a" \
            GMP_CFLAGS="-I$DEPS/include" GMP_LIBS="$DEPS/lib/libgmp.a" \
            LIBS="$DEPS/lib/libnettle.a $DEPS/lib/libgmp.a" # para sus pruebas de enlace de hogweed
        make -f Makefile -j"$JOBS" # su GNUmakefile de gnulib quiere un checkout de git
        make -f Makefile install
    fi
    if [ ! -f "$DEPS/lib/libMoltenVK.dylib" ]; then
        step "MoltenVK $MOLTENVK"
        mkdir -p "$WORK/addons"
        cd "$WORK/addons"
        f=MoltenVK-macos-$MOLTENVK.tar
        [ -f "$f" ] || curl -fL -o "$f" "https://github.com/KhronosGroup/MoltenVK/releases/download/$MOLTENVK/MoltenVK-macos.tar"
        echo "$MOLTENVK_SHA256  $f" | shasum -a 256 -c -
        tar -xf "$f" MoltenVK/MoltenVK/dynamic/dylib/macOS/libMoltenVK.dylib
        lipo -thin x86_64 MoltenVK/MoltenVK/dynamic/dylib/macOS/libMoltenVK.dylib -output "$DEPS/lib/libMoltenVK.dylib"
        rm -rf MoltenVK
    fi
    if [ ! -f "$DEPS/lib/libfreetype.6.dylib" ]; then
        step "freetype"
        cmake -S "$SRC/freetype" -B "$WORK/build-freetype" -DCMAKE_BUILD_TYPE=Release \
            -DCMAKE_OSX_ARCHITECTURES=x86_64 -DCMAKE_INSTALL_PREFIX="$DEPS" -DCMAKE_INSTALL_NAME_DIR="$DEPS/lib" \
            -DBUILD_SHARED_LIBS=ON -DFT_DISABLE_BZIP2=ON -DFT_DISABLE_PNG=ON -DFT_DISABLE_HARFBUZZ=ON -DFT_DISABLE_BROTLI=ON
        cmake --build "$WORK/build-freetype" -j"$JOBS"
        cmake --install "$WORK/build-freetype"
    fi
}

# Sin X11/Wayland (es macOS), ni SDL, GStreamer o CUPS por ahora: ver «Pendiente» en docs/MOTOR.md.
wine() {
    step "Wine (CrossOver $VERSION)"
    mkdir -p "$WORK/build-wine"
    cd "$WORK/build-wine"
    [ -f Makefile ] || "$SRC/wine/configure" $X86 --prefix="$DIST" \
        --enable-archs=i386,x86_64 --disable-tests \
        --with-mingw --with-coreaudio --with-freetype --with-gnutls --with-vulkan \
        --without-x --without-wayland --without-sdl --without-gstreamer --without-cups --without-ffmpeg \
        FREETYPE_CFLAGS="-I$DEPS/include/freetype2" FREETYPE_LIBS="-L$DEPS/lib -lfreetype" \
        ac_cv_lib_soname_gnutls=@rpath/libgnutls.30.dylib ac_cv_lib_soname_freetype=@rpath/libfreetype.6.dylib \
        ac_cv_lib_soname_MoltenVK=@rpath/libMoltenVK.dylib \
        LDFLAGS="$LDFLAGS -Wl,-headerpad_max_install_names -Wl,-rpath,@loader_path/../../ -Wl,-rpath,@loader_path/../lib"
    # Arriba: freetype2.pc pide zlib.pc, que macOS no trae. Y Wine hace dlopen() del nombre que detecta;
    # sin ruta, dyld no mira el rpath. Con @rpath lo resuelve el rpath de cada .so, esté donde esté el motor.
    make -j"$JOBS"
    rm -rf "$DIST"
    make install-lib
    # En deps/ llevan ruta absoluta (las herramientas del build, como sfnt2fon, enlazan freetype).
    # La copia del motor lleva @rpath: las .so de lib/wine/x86_64-unix la abren con rpath @loader_path/../../ = lib/.
    for l in libgnutls.30.dylib libfreetype.6.dylib libMoltenVK.dylib; do
        cp "$DEPS/lib/$l" "$DIST/lib/"
        install_name_tool -id "@rpath/$l" "$DIST/lib/$l"
    done
}

# Capas gráficas (graphics_layer en src/core.rs). Wine busca cada builtin primero en lib/wine y luego en
# WINEDLLPATH: las DLL que chocan salen de lib/wine y Barrica elige la capa del bottle con WINEDLLPATH.
layers() {
    step "Capas gráficas: WineD3D y DXMT $DXMT"
    L=$DIST/lib/wine-layers
    rm -rf "$L"
    for a in i386 x86_64; do
        mkdir -p "$L/wined3d/$a-windows"
        for d in dxgi d3d10 d3d10core d3d11 d3d12 d3d12core; do # d3d10: D3DMetal trae el suyo
            if [ -f "$DIST/lib/wine/$a-windows/$d.dll" ]; then
                mv "$DIST/lib/wine/$a-windows/$d.dll" "$L/wined3d/$a-windows/"
            fi
        done
    done
    mkdir -p "$WORK/addons"
    cd "$WORK/addons"
    f=dxmt-$DXMT-builtin.tar.gz
    [ -f "$f" ] || curl -fL -O "https://github.com/3Shain/dxmt/releases/download/$DXMT/$f"
    echo "$DXMT_SHA256  $f" | shasum -a 256 -c -
    tar -xzf "$f" -C "$L"
    mv "$L/$DXMT" "$L/dxmt"
    # winemetal (el puente a Metal) no choca con nada y su .so busca winemac.so y ntdll.so en @loader_path:
    # va con el resto de Wine. En la capa quedan d3d11, dxgi, d3d10core y nvapi64/nvngx (DLSS → MetalFX).
    for a in i386 x86_64; do mv "$L/dxmt/$a-windows/winemetal.dll" "$DIST/lib/wine/$a-windows/"; done
    mv "$L/dxmt/x86_64-unix/winemetal.so" "$DIST/lib/wine/x86_64-unix/"
    rmdir "$L/dxmt/x86_64-unix"
}

# Wine Mono (.NET) y Gecko (HTML): sin ellos, cada bottle nuevo pregunta si descargarlos.
addons() {
    step "Wine Mono $MONO y Gecko $GECKO"
    mkdir -p "$WORK/addons" "$DIST/share/wine/mono" "$DIST/share/wine/gecko"
    cd "$WORK/addons"
    for f in wine-mono/$MONO/wine-mono-$MONO-x86.tar.xz wine-gecko/$GECKO/wine-gecko-$GECKO-x86.tar.xz \
        wine-gecko/$GECKO/wine-gecko-$GECKO-x86_64.tar.xz; do
        [ -f "${f##*/}" ] || curl -fL -O "https://dl.winehq.org/wine/$f"
    done
    tar -xJf "wine-mono-$MONO-x86.tar.xz" -C "$DIST/share/wine/mono"
    for a in x86 x86_64; do tar -xJf "wine-gecko-$GECKO-$a.tar.xz" -C "$DIST/share/wine/gecko"; done
}

case "${1:-all}" in
    wine)
        patches
        wine
        layers
        addons
        ;;
    all)
        fetch
        patches
        deps
        wine
        layers
        addons
        ;;
    *)
        echo "uso: $0 [all|wine]" >&2
        exit 2
        ;;
esac
step "Motor listo en $DIST ($(du -sh "$DIST" | cut -f1))"
