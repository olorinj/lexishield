#!/usr/bin/env bash
set -e

# ==============================================================================
# GESTIÓN DE ENTORNO VAGRANT (HOST)
# ==============================================================================

is_inside_vagrant=false
if [ -d "/vagrant" ] && [ "$PWD" = "/vagrant" ]; then
    is_inside_vagrant=true
fi

native_mode=false
do_update=false
for arg in "$@"; do
    if [ "$arg" == "--native" ]; then
        native_mode=true
    elif [ "$arg" == "-u" ] || [ "$arg" == "--update" ] || [ "$arg" == "--provision" ]; then
        do_update=true
    fi
done

if [ "$is_inside_vagrant" = false ] && [ "$native_mode" = false ]; then
    if ! command -v vagrant &> /dev/null; then
        echo -e "\n\e[33m[AVISO] Vagrant no esta instalado en el Host.\e[0m"
        echo -e "\e[90mPara compilar de forma aislada y multiplataforma, instale Vagrant y VirtualBox.\e[0m"
        echo -e "\e[36mEjecutando en modo nativo local con las herramientas del sistema...\e[0m\n"
        native_mode=true
    else
        # Garantizar que VAGRANT_HOME apunte a un directorio con permisos de lectura y escritura
        if [ -z "$VAGRANT_HOME" ]; then
            if [ -d "$HOME/.vagrant.d" ] && [ ! -w "$HOME/.vagrant.d" ]; then
                echo -e "\e[33m[AVISO] '$HOME/.vagrant.d' no tiene permisos de escritura. Redirigiendo VAGRANT_HOME a './.vagrant.d'...\e[0m"
                mkdir -p "$PWD/.vagrant.d"
                export VAGRANT_HOME="$PWD/.vagrant.d"
            elif [ ! -d "$HOME/.vagrant.d" ]; then
                mkdir -p "$HOME/.vagrant.d" 2>/dev/null || {
                    mkdir -p "$PWD/.vagrant.d"
                    export VAGRANT_HOME="$PWD/.vagrant.d"
                }
            fi
        fi

        if [ "$do_update" = true ]; then
            echo -e "\e[36mIniciando y actualizando entorno de compilacion (Vagrant --provision)...\e[0m"
            vagrant up --provision
            echo -e "\e[36mActualizando toolchain de Rust dentro de la maquina virtual...\e[0m"
            vagrant ssh -c "rustup update && sudo -i rustup update >/dev/null 2>&1 || true"
        else
            echo -e "\e[36mIniciando entorno de compilacion centralizado en Vagrant...\e[0m"
            vagrant up
        fi

        echo -e "\e[36mEjecutando compilacion dentro de la maquina virtual...\e[0m"
        vagrant ssh -c "cd /vagrant && dos2unix compilar.sh >/dev/null 2>&1 && bash ./compilar.sh $*"
        exit_code=$?
        
        # Apagar la maquina virtual si no se pidio mantenerla (-k / --keep-vm)
        if [[ ! " $* " =~ " -k " ]] && [[ ! " $* " =~ " --keep-vm " ]]; then
            echo -e "\e[90mApagando la maquina virtual para liberar recursos de memoria/CPU...\e[0m"
            vagrant halt
        fi
        exit $exit_code
    fi
fi

# ==============================================================================
# MOTOR DE COMPILACIÓN (Se ejecuta dentro de la VM o en modo nativo)
# ==============================================================================

directorio_original=$(pwd)
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

trap 'cd "$directorio_original"' EXIT
cd "$script_dir"

do_clean=false
keep_intermediate=false
compilar_win=false
compilar_linux=false
compilar_mac=false
especifico=false

# Parsear argumentos
while [[ $# -gt 0 ]]; do
    case "$1" in
        -w|--windows|--win)
            compilar_win=true
            especifico=true
            shift
            ;;
        -l|--linux)
            compilar_linux=true
            especifico=true
            shift
            ;;
        -m|--mac|--macos)
            compilar_mac=true
            especifico=true
            shift
            ;;
        -a|--all)
            compilar_win=true
            compilar_linux=true
            compilar_mac=true
            especifico=true
            shift
            ;;
        -c|--clean|--clean-all)
            do_clean=true
            shift
            ;;
        -u|--update|--provision)
            do_update=true
            shift
            ;;
        -k|--keep-intermediate|--keep-vm)
            keep_intermediate=true
            shift
            ;;
        --native)
            shift
            ;;
        *)
            echo "Argumento no reconocido: $1"
            echo "Uso: ./compilar.sh [-w|--windows] [-l|--linux] [-m|--mac] [-a|--all] [-c|--clean] [-u|--update] [-k|--keep-vm] [--native]"
            exit 1
            ;;
    esac
done

if [ "$do_update" = true ]; then
    echo -e "\e[36mActualizando toolchain de Rust...\e[0m"
    rustup update 2>/dev/null || true
fi

# Optimizacion de I/O en Vagrant: Compilar en disco rapido /tmp para evitar cuello de botella de VirtualBox Shared Folders
if [ "$is_inside_vagrant" = true ]; then
    export CARGO_TARGET_DIR="/tmp/lexishield-target"
    mkdir -p "$CARGO_TARGET_DIR"
else
    export CARGO_TARGET_DIR="${script_dir}/target"
fi

if [ "$do_clean" = true ]; then
    echo -e "\e[36mRealizando limpieza completa (cargo clean)...\e[0m"
    if [ "$is_inside_vagrant" = true ]; then
        sudo rm -rf "$CARGO_TARGET_DIR"
    fi
    cargo clean
fi

echo -e "\e[36mVerificando codigo y formateo (Release)...\e[0m"
cargo clippy --release --fix --allow-dirty -- -D warnings
cargo fmt

archivos_compilados=()

# 1. Compilación para Linux (x86_64)
if [ "$especifico" = false ]; then
    compilar_linux=true
fi

if [ "$compilar_linux" = true ]; then
    echo -e "\n\e[36m[1/3] Compilando version Release para Linux (x86_64-unknown-linux-gnu)...\e[0m"
    cargo build --release --target x86_64-unknown-linux-gnu
    
    bin_src="$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/lexishield"
    if [ ! -f "$bin_src" ]; then
        bin_src="$CARGO_TARGET_DIR/release/lexishield"
    fi
    
    if [ -f "$bin_src" ]; then
        mkdir -p dist
        cp -f "$bin_src" "./dist/lexishield-linux-amd64"
        archivos_compilados+=("./dist/lexishield-linux-amd64")
    fi
fi

# 2. Compilacion cruzada para Windows (.exe con MinGW)
if [ "$especifico" = false ]; then
    read -p $'\n¿Desea compilar tambien para Windows (x86_64 .exe)? (s/N) ' resp_win
    if [[ "$resp_win" =~ ^[sS]$ ]]; then
        compilar_win=true
    fi
fi

if [ "$compilar_win" = true ]; then
    echo -e "\n\e[36m[2/3] Compilando version Release para Windows (x86_64-pc-windows-gnu)...\e[0m"
    rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
    cargo build --target x86_64-pc-windows-gnu --release
    
    bin_win="$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release/lexishield.exe"
    if [ -f "$bin_win" ]; then
        mkdir -p dist
        cp -f "$bin_win" "./dist/lexishield-windows-amd64.exe"
        archivos_compilados+=("./dist/lexishield-windows-amd64.exe")
    fi
fi

# 3. Compilacion cruzada para macOS (usando Docker con crazymax/osxcross)
if [ "$especifico" = false ]; then
    read -p $'\n¿Desea compilar tambien para macOS (x86_64 y ARM64)? (s/N) ' resp_mac
    if [[ "$resp_mac" =~ ^[sS]$ ]]; then
        compilar_mac=true
    fi
fi

if [ "$compilar_mac" = true ]; then
    echo -e "\n\e[36m[3/3] Compilando version Release para macOS (Intel y Apple Silicon)...\e[0m"
    if ! command -v docker &> /dev/null; then
        echo -e "\e[31m[ERROR] Docker no esta disponible para la compilacion de macOS.\e[0m"
    else
        echo -e "\e[90mConstruyendo contenedor de compilacion macOS (crazymax/osxcross)...\e[0m"
        docker build -t lexishield-osxcross -f build/Dockerfile.osxcross .
        
        echo -e "\e[36m -> Compilando macOS Intel (x86_64-apple-darwin)...\e[0m"
        docker run --rm -v "$(pwd):/src" -v "$CARGO_TARGET_DIR:/src/target" lexishield-osxcross cargo build --target x86_64-apple-darwin --release
        
        echo -e "\e[36m -> Compilando macOS Apple Silicon (aarch64-apple-darwin)...\e[0m"
        docker run --rm -v "$(pwd):/src" -v "$CARGO_TARGET_DIR:/src/target" lexishield-osxcross cargo build --target aarch64-apple-darwin --release

        mkdir -p dist
        if [ -f "$CARGO_TARGET_DIR/x86_64-apple-darwin/release/lexishield" ]; then
            cp -f "$CARGO_TARGET_DIR/x86_64-apple-darwin/release/lexishield" "./dist/lexishield-darwin-amd64"
            archivos_compilados+=("./dist/lexishield-darwin-amd64")
        fi
        if [ -f "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/lexishield" ]; then
            cp -f "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/lexishield" "./dist/lexishield-darwin-arm64"
            archivos_compilados+=("./dist/lexishield-darwin-arm64")
        fi
    fi
fi

# Limpieza de temporales si no se requiere conservarlos
if [ "$keep_intermediate" = false ]; then
    echo -e "\n\e[36mLimpiando ficheros intermedios de compilacion...\e[0m"
    if [ "$is_inside_vagrant" = true ]; then
        sudo rm -rf "$CARGO_TARGET_DIR"
    fi
    cargo clean
    rm -rf target/
fi

echo -e "\n\e[32m========================================================\e[0m"
echo -e "\e[32mCompilacion finalizada exitosamente.\e[0m"
echo -e "\e[32mBinarios generados:\e[0m"
for bin in "${archivos_compilados[@]}"; do
    if [ -f "$bin" ]; then
        echo -e "  -> \e[97m$(basename "$bin")\e[0m (\e[90m$(ls -lh "$bin" | awk '{print $5}')\e[0m)"
    fi
done
echo -e "\e[32m========================================================\e[0m"
