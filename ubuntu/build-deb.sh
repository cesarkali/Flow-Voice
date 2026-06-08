#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERSION="$(python3 -c "import sys; sys.path.insert(0, '$PROJECT_ROOT'); from version import VERSION; print(VERSION)")"
ARCH="amd64"
PKG_NAME="flowvoice"
PROJECT_DIST_DIR="$SCRIPT_DIR/dist"

is_vbox_shared_folder() {
    local target_path="$1"
    if command -v findmnt >/dev/null 2>&1; then
        findmnt -T "$target_path" -no FSTYPE 2>/dev/null | grep -qx vboxsf
        return $?
    fi
    [[ "$target_path" == /media/sf_* ]]
}

# Pastas compartilhadas do VirtualBox (/media/sf_*) não suportam symlinks do venv/PyInstaller.
if is_vbox_shared_folder "$PROJECT_ROOT" || [[ "${FLOWVOICE_BUILD_LOCAL:-}" == "1" ]]; then
    BUILD_DIR="${FLOWVOICE_BUILD_DIR:-/tmp/flowvoice-build-${USER:-build}}"
    echo "==> Pasta compartilhada detectada: build temporário em ${BUILD_DIR}"
else
    BUILD_DIR="$SCRIPT_DIR/build"
fi

STAGING="$BUILD_DIR/${PKG_NAME}_${VERSION}_${ARCH}"

echo "==> FlowVoice: gerando pacote .deb para Ubuntu (${VERSION}, ${ARCH})"

if ! command -v python3 >/dev/null 2>&1; then
    echo "Erro: python3 não encontrado. Instale com: sudo apt install python3 python3-venv python3-pip"
    exit 1
fi

if ! command -v dpkg-deb >/dev/null 2>&1; then
    echo "Erro: dpkg-deb não encontrado. Instale com: sudo apt install dpkg-dev"
    exit 1
fi

cd "$PROJECT_ROOT"

mkdir -p "$BUILD_DIR" "$PROJECT_DIST_DIR"
rm -rf "$STAGING"

echo "==> Criando ambiente virtual e instalando dependências..."
python3 -m venv --copies "$BUILD_DIR/venv"
# shellcheck disable=SC1091
source "$BUILD_DIR/venv/bin/activate"
pip install --upgrade pip
pip install -r requirements-linux.txt
pip install pyinstaller

echo "==> Compilando executável com PyInstaller..."
PYINSTALLER_ARGS=(
    --noconsole
    --onedir
    --name flowvoice
    --distpath "$BUILD_DIR/pyinstaller-dist"
    --workpath "$BUILD_DIR/pyinstaller-work"
    --specpath "$BUILD_DIR"
    --paths "$PROJECT_ROOT"
    --hidden-import="pynput.keyboard._xorg"
    --hidden-import="version"
)

if [ -f "$PROJECT_ROOT/icon.png" ]; then
    PYINSTALLER_ARGS+=(--icon="$PROJECT_ROOT/icon.png" --add-data "$PROJECT_ROOT/icon.png:.")
else
    echo "==> Aviso: icon.png não encontrado; compilando sem ícone personalizado."
fi

pyinstaller "${PYINSTALLER_ARGS[@]}" main.py

echo "==> Montando estrutura do pacote .deb..."
mkdir -p "$STAGING/DEBIAN"
mkdir -p "$STAGING/opt/flowvoice"
mkdir -p "$STAGING/usr/share/applications"
mkdir -p "$STAGING/usr/share/icons/hicolor/256x256/apps"

cp -a "$BUILD_DIR/pyinstaller-dist/flowvoice/." "$STAGING/opt/flowvoice/"
cp "$SCRIPT_DIR/debian/flowvoice.desktop" "$STAGING/usr/share/applications/"
chmod 644 "$STAGING/usr/share/applications/flowvoice.desktop"

if [ -f "$PROJECT_ROOT/icon.png" ]; then
    cp "$PROJECT_ROOT/icon.png" "$STAGING/usr/share/icons/hicolor/256x256/apps/flowvoice.png"
    chmod 644 "$STAGING/usr/share/icons/hicolor/256x256/apps/flowvoice.png"
fi

sed "s/@VERSION@/$VERSION/g" "$SCRIPT_DIR/debian/control.template" > "$STAGING/DEBIAN/control"
cp "$SCRIPT_DIR/debian/postinst" "$STAGING/DEBIAN/postinst"
cp "$SCRIPT_DIR/debian/prerm" "$STAGING/DEBIAN/prerm"
chmod 755 "$STAGING/DEBIAN/postinst"
chmod 755 "$STAGING/DEBIAN/prerm"

echo "==> Empacotando..."
DEB_FILE="$PROJECT_DIST_DIR/${PKG_NAME}_${VERSION}_${ARCH}.deb"
dpkg-deb --root-owner-group --build "$STAGING" "$DEB_FILE"

echo
echo "Pacote gerado com sucesso:"
echo "  $DEB_FILE"
echo
echo "Instalação no Ubuntu:"
echo "  sudo apt install ./ubuntu/dist/${PKG_NAME}_${VERSION}_${ARCH}.deb"
