#!/usr/bin/env bash
# Builds the Trindle Scan plugin into OUT (first argument, default plugins-out): OUT/<Id>/, .tar.zst, .description.txt and .svg.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
out="${1:-plugins-out}"
id="org.trindle.Scan"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
plugin="$out/$id"
permissions="${SCAN_PERMISSIONS:-home-network;files;usb-devices}"

git='patch."https://github.com/trindle-os/trindle-toolkit"'
if [ -n "${TOOLKIT_ROOT:-}" ]; then
    use=(--config "$git.trindle-toolkit.path=\"$TOOLKIT_ROOT\"" --config "$git.trindle-common.path=\"$TOOLKIT_ROOT/trindle-common\"")
else
    use=(--locked)
fi
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"
target="${SCAN_TARGET_DIR:-$root/target/shipped}"
remap="--remap-path-prefix=$HOME=/home --remap-path-prefix=$rustup_home=/rustup --remap-path-prefix=$cargo_home=/cargo --remap-path-prefix=$root=/src --remap-path-prefix=$target=/target"
[ -z "${TOOLKIT_ROOT:-}" ] || remap="$remap --remap-path-prefix=$TOOLKIT_ROOT=/toolkit"
RUSTFLAGS="${RUSTFLAGS:-} $remap" cargo build --release --offline "${use[@]}" --manifest-path "$root/Cargo.toml" --target-dir "$target"

needs="$(ldd "$target/release/trindle-scan" | awk '/=>/ {print $1}' | sort | tr '\n' ' ')"
for lib in $needs; do
    case "$lib" in
        libc.so.6|libffi.so.8|libgcc_s.so.1|libm.so.6|libwayland-client.so.0|libxkbcommon.so.0) ;;
        *) echo "trindle-scan now needs $lib: carry it in lib/ before shipping ($needs)" >&2; exit 1 ;;
    esac
done

rm -rf "$plugin" "$out/$id.tar.zst"
install -d "$plugin/bin" "$plugin/icons"
install -m 0755 "$target/release/trindle-scan" "$plugin/bin/trindle-scan"
install -m 0644 "$here/icons/scan-ring.svg" "$plugin/icons/scan-ring.svg"
if LC_ALL=C grep -aqF -e "/home/" -e "$HOME" "$plugin/bin/trindle-scan"; then
    echo "the built program holds a path of this computer; check the path remap" >&2
    exit 1
fi
install -m 0644 "$root/LICENSE" "$plugin/LICENSE"
install -d "$plugin/share/licences"
install -m 0644 "$root/share/licences/THIRD-PARTY.txt" "$plugin/share/licences/THIRD-PARTY.txt"

if [ -n "${TESSERACT_ROOT:-}" ]; then
    install -d "$plugin/lib" "$plugin/share/tessdata"
    install -m 0755 "$TESSERACT_ROOT/usr/bin/tesseract" "$plugin/lib/tesseract-real"
    cp -r "$TESSERACT_ROOT"/usr/share/tesseract-ocr/*/tessdata/configs "$plugin/share/tessdata/"
    for f in "$TESSERACT_ROOT"/usr/share/tesseract-ocr/*/tessdata/*.traineddata; do
        install -m 0644 "$f" "$plugin/share/tessdata/"
    done
    todo="$plugin/lib/tesseract-real"; seen=" "
    while [ -n "$todo" ]; do
        elf="${todo%% *}"; todo="${todo#"$elf"}"; todo="${todo# }"
        for name in $(objdump -p "$elf" | awk '/NEEDED/ {print $2}'); do
            case "$seen" in *" $name "*) continue ;; esac
            seen="$seen$name "
            case "$name" in
                libc.so.6|libm.so.6|libgcc_s.so.1|libstdc++.so.6|libresolv.so.2|libdl.so.2|libpthread.so.0) continue ;;
            esac
            found="$(ldconfig -p | awk -v n="$name" '$1 == n {print $NF; exit}')"
            [ -n "$found" ] || { echo "cannot find $name" >&2; exit 1; }
            real="$(readlink -f "$found")"
            install -m 0644 "$real" "$plugin/lib/$name"
            pkg="$(dpkg -S "$real" 2>/dev/null | head -1 | cut -d: -f1)"
            if [ -n "$pkg" ] && [ -f "/usr/share/doc/$pkg/copyright" ]; then
                cp "/usr/share/doc/$pkg/copyright" "$plugin/share/licences/$pkg-copyright"
            else
                echo "no copyright file for $name (package: ${pkg:-unknown})" >&2
                exit 1
            fi
            todo="$todo $plugin/lib/$name"
        done
    done
    cat > "$plugin/bin/tesseract" <<'WRAP'
here=$(dirname "$(readlink -f "$0")")
LD_LIBRARY_PATH="$here/../lib" exec "$here/../lib/tesseract-real" "$@"
WRAP
    chmod 0755 "$plugin/bin/tesseract"
    for d in "$TESSERACT_ROOT"/usr/share/doc/tesseract-ocr*; do
        cp "$d/copyright" "$plugin/share/licences/$(basename "$d")-copyright"
    done
    echo "Tesseract (Apache-2.0) and the libraries it needs are carried in lib/; each package's copyright file is in this folder." > "$plugin/share/licences/README"
fi

cat > "$plugin/plugin.ini" <<INI
[Plugin]
Interface=1
Id=$id
Name=Scan
Version=$version
Summary=Scan pages from a network scanner, tidy them up and save them as PDF or pictures.
Permissions=$permissions

[Ring]
Icon=icons/scan-ring.svg
Label=Scan
Run=bin/trindle-scan
INI

install -m 0644 "$here/icons/scan-ring.svg" "$out/$id.svg"
cat > "$out/$id.description.txt" <<TXT
Scan finds a scanner on your home network by itself, or you can type its address, and scans pages from the glass or the feeder. Pages appear as pictures you can turn, crop, straighten and reorder, then save as one PDF or as pictures. Profiles keep your favourite settings, and a batch can scan stack after stack and save each as it finishes. Scan looks for scanners only while its window is open.
TXT
tar --zstd -cf "$out/$id.tar.zst" -C "$out" "$id"
echo "$plugin"
