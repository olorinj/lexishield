#!/bin/bash
set -e

echo "==> Compilando LexiShield v2.0 (Tauri GUI + Rust CLI)..."
cd ui
echo "Instalando dependencias Web..."
npm install
cd ..

echo "Empaquetando Frontend (Svelte) y compilando Backend (Rust)..."
export APPIMAGE_EXTRACT_AND_RUN=1
npx --prefix ui tauri build --no-bundle

mkdir -p dist
rm -f dist/lexishield-tauri-linux
cp src-tauri/target/release/lexishield-gui dist/lexishield-tauri-linux
chmod +x dist/lexishield-tauri-linux

echo ""
echo "¡Compilación completada con éxito!"
echo "Puedes ejecutar la versión final aquí:"
echo "  ./dist/lexishield-tauri-linux"
