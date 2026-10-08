#!/usr/bin/env bash
set -e

# ==============================================================================
# GESTIÓN DE ENTORNO VAGRANT (HOST)
# ==============================================================================

is_inside_vagrant=false
if [ -d "/vagrant" ] && [ "$PWD" = "/vagrant" ]; then
    is_inside_vagrant=true
fi

show_menu=false
if [ $# -eq 0 ] && [ "$is_inside_vagrant" = false ]; then
    show_menu=true
fi

for arg in "$@"; do
    if [ "$arg" == "-i" ] || [ "$arg" == "--menu" ] || [ "$arg" == "-menu" ]; then
        show_menu=true
        break
    fi
done

if [ "$show_menu" = true ]; then
    echo -e "\n\e[36m========================================================\e[0m"
    echo -e "\e[33m         LexiShield - Menú de Compilación\e[0m"
    echo -e "\e[36m========================================================\e[0m"
    echo -e " \e[97m1)\e[0m Todo (Linux, macOS, Windows CLI + GUI) -> \e[90m[VMs secuenciales]\e[0m"
    echo -e " \e[97m2)\e[0m Solo Linux (CLI + GUI)                 -> \e[90m[builder-linux]\e[0m"
    echo -e " \e[97m3)\e[0m Solo Windows (CLI + GUI)               -> \e[90m[builder-windows]\e[0m"
    echo -e " \e[97m4)\e[0m Solo macOS (Intel + ARM64)             -> \e[90m[builder-linux con Docker]\e[0m"
    echo -e " \e[97m5)\e[0m Solo CLI (Linux + Windows)             -> \e[90m[Sin interfaces gráficas]\e[0m"
    echo -e " \e[97m6)\e[0m Modo Nativo Local (Host actual)        -> \e[90m[Sin usar Vagrant]\e[0m"
    echo -e " \e[97m0)\e[0m Salir"
    echo -e "\e[36m========================================================\e[0m"

    read -r -p "Seleccione una opción [0-6]: " opcion_menu

    selected_args=()
    case "$opcion_menu" in
        1)
            selected_args+=("-a")
            ;;
        2)
            selected_args+=("-l" "-g")
            ;;
        3)
            selected_args+=("-w" "-g")
            ;;
        4)
            selected_args+=("-m")
            ;;
        5)
            selected_args+=("-l" "-w")
            ;;
        6)
            selected_args+=("--native")
            read -r -p "¿Desea compilar también la interfaz gráfica (GUI)? [s/N]: " resp_gui
            if [[ "$resp_gui" =~ ^[sSyY]$ ]]; then
                selected_args+=("-g")
            fi
            ;;
        0)
            echo -e "\e[33mOperación cancelada.\e[0m"
            exit 0
            ;;
        *)
            echo -e "\e[31mOpción no válida: '$opcion_menu'.\e[0m"
            exit 1
            ;;
    esac

    read -r -p "¿Desea realizar una limpieza previa (cargo clean)? [s/N]: " resp_clean
    if [[ "$resp_clean" =~ ^[sSyY]$ ]]; then
        selected_args+=("-c")
    fi

    read -r -p "¿Desea instalar/actualizar herramientas en la VM (Vagrant provision)? [s/N]: " resp_prov
    if [[ "$resp_prov" =~ ^[sSyY]$ ]]; then
        selected_args+=("-u")
    fi

    # Preservar flags adicionales pasados al llamar al script (-u, -k)
    for arg in "$@"; do
        if [ "$arg" == "-u" ] || [ "$arg" == "--update" ] || [ "$arg" == "--provision" ]; then
            selected_args+=("-u")
        elif [ "$arg" == "-k" ] || [ "$arg" == "--keep-vm" ] || [ "$arg" == "--keep-intermediate" ]; then
            selected_args+=("-k")
        fi
    done

    set -- "${selected_args[@]}"
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

        need_linux=false
        need_win=false

        if [[ " $* " =~ " -w " ]] || [[ " $* " =~ " --windows " ]] || [[ " $* " =~ " --win " ]]; then
            need_win=true
        fi
        if [[ " $* " =~ " -l " ]] || [[ " $* " =~ " --linux " ]] || [[ " $* " =~ " -m " ]] || [[ " $* " =~ " --mac " ]] || [[ " $* " =~ " --macos " ]]; then
            need_linux=true
        fi
        if [[ " $* " =~ " -a " ]] || [[ " $* " =~ " --all " ]]; then
            need_linux=true
            need_win=true
        fi
        # Si no se pasó ningún target específico (-l, -m, -w, -a), por defecto es Linux
        if [ "$need_linux" = false ] && [ "$need_win" = false ]; then
            need_linux=true
        fi

        exit_code=0

        # 1. Compilar en builder-linux si es requerido
        if [ "$need_linux" = true ]; then
            if [ "$do_update" = true ]; then
                echo -e "\e[36mIniciando y actualizando entorno Linux/macOS (Vagrant builder-linux --provision)...\e[0m"
                vagrant up builder-linux --provision
                echo -e "\e[36mActualizando toolchain de Rust dentro de builder-linux...\e[0m"
                vagrant ssh builder-linux -c "rustup update && sudo -i rustup update >/dev/null 2>&1 || true"
            else
                echo -e "\e[36mIniciando entorno Linux/macOS en Vagrant (builder-linux)...\e[0m"
                vagrant up builder-linux
            fi

            echo -e "\e[36mEjecutando compilacion dentro de builder-linux...\e[0m"
            vagrant ssh builder-linux -c "if ! mountpoint -q /vagrant; then sudo mkdir -p /vagrant && sudo mount -t vboxsf -o uid=\$(id -u vagrant),gid=\$(id -g vagrant) vagrant /vagrant 2>/dev/null || true; fi; cd /vagrant && dos2unix compilar.sh >/dev/null 2>&1 && bash ./compilar.sh $*"
            exit_code=$?
            
            # Apagar builder-linux para liberar recursos
            if [[ ! " $* " =~ " -k " ]] && [[ ! " $* " =~ " --keep-vm " ]]; then
                echo -e "\e[90mApagando la maquina virtual builder-linux para liberar memoria/CPU...\e[0m"
                vagrant halt builder-linux
            fi
        fi

        # 2. Compilar en builder-windows si es requerido
        if [ "$need_win" = true ] && [ $exit_code -eq 0 ]; then
            if [ "$do_update" = true ]; then
                echo -e "\n\e[36mIniciando y actualizando entorno Windows (Vagrant builder-windows --provision)...\e[0m"
                vagrant up builder-windows --provision
            else
                echo -e "\n\e[36mIniciando entorno Windows en Vagrant (builder-windows)...\e[0m"
                vagrant up builder-windows
            fi

            echo -e "\e[36mEjecutando compilacion dentro de builder-windows...\e[0m"
            vagrant winrm builder-windows -c "powershell -ExecutionPolicy Bypass -File C:\\vagrant\\compilar-win.ps1 $*"
            exit_code=$?

            # Apagar builder-windows para liberar recursos
            if [[ ! " $* " =~ " -k " ]] && [[ ! " $* " =~ " --keep-vm " ]]; then
                echo -e "\e[90mApagando la maquina virtual builder-windows para liberar memoria/CPU...\e[0m"
                vagrant halt builder-windows
            fi
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
        -g|--gui)
            compilar_gui=true
            especifico=true
            shift
            ;;
        -a|--all)
            compilar_win=true
            compilar_linux=true
            compilar_mac=true
            compilar_gui=true
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
        -i|--menu|-menu)
            shift
            ;;
        --native)
            shift
            ;;
        *)
            echo "Argumento no reconocido: $1"
            echo "Uso: ./compilar.sh [-i|--menu] [-w|--windows] [-l|--linux] [-m|--mac] [-g|--gui] [-a|--all] [-c|--clean] [-u|--update] [-k|--keep-vm] [--native]"
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

# 2. Información sobre compilación de Windows
if [ "$compilar_win" = true ]; then
    echo -e "\n\e[33m[INFO] La compilacion para Windows ahora se gestiona de forma nativa en la VM 'builder-windows' (MSVC/Tauri).\e[0m"
fi

# 3. Compilacion cruzada para macOS (usando Docker con crazymax/osxcross)
if [ "$especifico" = false ]; then
    read -p $'\n¿Desea compilar tambien para macOS (x86_64 y ARM64)? (s/N) ' resp_mac
    if [[ "$resp_mac" =~ ^[sS]$ ]]; then
        compilar_mac=true
    fi
fi

if [ "$compilar_mac" = true ]; then
    echo -e "\n\e[36m[2/3] Compilando version Release para macOS (Intel y Apple Silicon)...\e[0m"
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

# 3. Compilacion de Interfaz Grafica (Tauri GUI Linux)
if [ "$compilar_gui" = true ]; then
    echo -e "\n\e[36m[3/3] Compilando Interfaz Grafica de Usuario (LexiShield Tauri GUI Linux)...\e[0m"
    if command -v npm &> /dev/null; then
        echo -e "\e[90mInstalando dependencias de UI y construyendo bundle frontend...\e[0m"
        (cd ui && npm install && npm run build)
        echo -e "\e[90mConstruyendo paquetes nativos (.deb, .rpm) con Tauri...\e[0m"
        export APPIMAGE_EXTRACT_AND_RUN=1
        export NO_STRIP=true
        ui/node_modules/.bin/tauri build --bundles deb,rpm
        
        mkdir -p dist
        # Buscar en CARGO_TARGET_DIR (usado en Vagrant) o en src-tauri/target
        bundle_dir="$CARGO_TARGET_DIR/release/bundle"
        if [ ! -d "$bundle_dir" ]; then
            bundle_dir="src-tauri/target/release/bundle"
        fi
        
        if [ -d "$bundle_dir" ]; then
            cp -r "$bundle_dir"/appimage/*.AppImage dist/ 2>/dev/null || true
            cp -r "$bundle_dir"/deb/*.deb dist/ 2>/dev/null || true
            cp -r "$bundle_dir"/rpm/*.rpm dist/ 2>/dev/null || true
        fi
        
        gui_bin="$CARGO_TARGET_DIR/release/lexishield-gui"
        if [ ! -f "$gui_bin" ]; then
            gui_bin="src-tauri/target/release/lexishield-gui"
        fi
        if [ -f "$gui_bin" ]; then
            cp -f "$gui_bin" "./dist/lexishield-gui-linux-amd64" 2>/dev/null || true
            archivos_compilados+=("./dist/lexishield-gui-linux-amd64")
        fi
        archivos_compilados+=("./dist/*.AppImage" "./dist/*.deb" "./dist/*.rpm")
    else
        echo -e "\e[33m[AVISO] 'npm' no encontrado en el PATH. Omitiendo compilacion de la GUI.\e[0m"
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
