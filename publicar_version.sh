#!/bin/bash
set -e

# Paleta de colores ANSI
COLOR_CYAN="\033[1;36m"
COLOR_GREEN="\033[1;32m"
COLOR_YELLOW="\033[1;33m"
COLOR_RED="\033[1;31m"
COLOR_GRAY="\033[0;90m"
COLOR_RESET="\033[0m"

# Función de ayuda
mostrar_ayuda() {
    echo -e "${COLOR_CYAN}================================================================================${COLOR_RESET}"
    echo -e "${COLOR_CYAN}  LexiShield - Asistente de Publicación de Versiones en GitHub${COLOR_RESET}"
    echo -e "${COLOR_CYAN}================================================================================${COLOR_RESET}"
    echo ""
    echo "Este script guía y automatiza el proceso de lanzamiento oficial en GitHub:"
    echo "  1. Comprueba la rama activa y el estado del repositorio Git local."
    echo "  2. Muestra un resumen claro de commits pendientes de subida y versión a etiquetar."
    echo "  3. Solicita confirmación explícita al usuario antes de interactuar con el servidor remoto."
    echo "  4. Sube la rama y la etiqueta a GitHub, activando el flujo automatizado de"
    echo "     GitHub Actions para compilar los instaladores nativos (.msi, .deb, .rpm, .dmg)."
    echo ""
    echo -e "${COLOR_CYAN}USO:${COLOR_RESET}"
    echo "  ./publicar_version.sh [OPCIONES] <VERSION>"
    echo ""
    echo -e "${COLOR_CYAN}ARGUMENTOS:${COLOR_RESET}"
    echo "  <VERSION>         Versión oficial a publicar (ej: v2.0.0 o 2.0.0)."
    echo "                    Si se omite el prefijo 'v', se añadirá automáticamente."
    echo ""
    echo -e "${COLOR_CYAN}OPCIONES:${COLOR_RESET}"
    echo "  -h, --help        Muestra esta pantalla de ayuda y sale."
    echo "  -f, --force       Fuerza la recreación de la etiqueta si ya existe (local o remota)."
    echo "  -y, --yes         Modo desatendido: confirma automáticamente sin pausar."
    echo "  -d, --dry-run     Modo simulación: verifica el estado pero no realiza git push."
    echo ""
    echo -e "${COLOR_CYAN}EJEMPLOS:${COLOR_RESET}"
    echo "  ./publicar_version.sh v2.0.0"
    echo "  ./publicar_version.sh 2.0.0"
    echo "  ./publicar_version.sh --force v2.0.0"
    echo "  ./publicar_version.sh --dry-run v2.0.0"
    echo ""
}

# Variables de control
VERSION=""
AUTO_CONFIRM=false
DRY_RUN=false
FORCE=false

# Procesar argumentos
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            mostrar_ayuda
            exit 0
            ;;
        -f|--force)
            FORCE=true
            shift
            ;;
        -y|--yes)
            AUTO_CONFIRM=true
            shift
            ;;
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -*)
            echo -e "${COLOR_RED}Error: Opción desconocida '$1'.${COLOR_RESET}"
            mostrar_ayuda
            exit 1
            ;;
        *)
            if [[ -z "$VERSION" ]]; then
                VERSION="$1"
            else
                echo -e "${COLOR_RED}Error: Argumento inesperado '$1'.${COLOR_RESET}"
                mostrar_ayuda
                exit 1
            fi
            shift
            ;;
    esac
done

# Validar si no se proporcionó versión
if [[ -z "$VERSION" ]]; then
    mostrar_ayuda
    echo -e "${COLOR_YELLOW}Por favor, especifica la versión a publicar (ej: ./publicar_version.sh v2.0.0).${COLOR_RESET}\n"
    exit 1
fi

# Normalizar versión con prefijo 'v'
if [[ ! "$VERSION" =~ ^v ]]; then
    echo -e "${COLOR_YELLOW}> Añadiendo prefijo 'v' a la versión: v${VERSION}${COLOR_RESET}"
    VERSION="v$VERSION"
fi

echo -e "\n${COLOR_CYAN}========================================================${COLOR_RESET}"
echo -e "${COLOR_CYAN}    Lanzamiento de LexiShield ${VERSION}${COLOR_RESET}"
echo -e "${COLOR_CYAN}========================================================${COLOR_RESET}"

# Obtener rama actual
RAMA_ACTUAL=$(git branch --show-current 2>/dev/null || echo "desconocida")
echo -e "${COLOR_GRAY}> Rama activa:${COLOR_RESET} ${COLOR_CYAN}${RAMA_ACTUAL}${COLOR_RESET}"

# Verificar si hay cambios locales sin confirmar
if [[ -n $(git status -s) ]]; then
    echo -e "${COLOR_RED}\n[!] Error: Tienes cambios locales pendientes de confirmación en Git:${COLOR_RESET}"
    git status -s
    echo -e "\n${COLOR_YELLOW}Por seguridad, confirma o descarta tus cambios antes de publicar una versión.${COLOR_RESET}"
    exit 1
fi

# Verificar si el tag ya existe local o remotamente
TAG_EXISTS=false
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    TAG_EXISTS=true
fi

if [[ "$TAG_EXISTS" = true ]]; then
    echo -e "${COLOR_YELLOW}\n[i] La etiqueta '${VERSION}' ya existe en el repositorio local.${COLOR_RESET}"
    
    RECREAR=false
    if [[ "$FORCE" = true ]]; then
        RECREAR=true
    else
        echo -ne "${COLOR_YELLOW}¿Deseas eliminar la versión anterior (local y en GitHub) y recrearla con el commit actual? [s/N]: ${COLOR_RESET}"
        read -r RESP_TAG
        if [[ "$RESP_TAG" =~ ^[sSyY]$ ]]; then
            RECREAR=true
        fi
    fi

    if [[ "$RECREAR" = true ]]; then
        if [[ "$DRY_RUN" = true ]]; then
            echo -e "${COLOR_GRAY}[Dry-Run] Se eliminaría la etiqueta local y remota '${VERSION}' para recrearla.${COLOR_RESET}"
        else
            echo -e "${COLOR_GRAY}> Eliminando etiqueta local '${VERSION}'...${COLOR_RESET}"
            git tag -d "$VERSION"
            echo -e "${COLOR_GRAY}> Eliminando etiqueta remota en GitHub (si existe)...${COLOR_RESET}"
            git push origin --delete "$VERSION" 2>/dev/null || true
            if command -v gh >/dev/null 2>&1; then
                gh release delete "$VERSION" -y 2>/dev/null || true
            fi
        fi
    else
        echo -e "${COLOR_RED}\nOperación cancelada. La etiqueta '${VERSION}' ya existe.${COLOR_RESET}"
        echo "Usa otra versión o ejecuta './publicar_version.sh --force ${VERSION}' para sobrescribirla."
        exit 1
    fi
fi

# Contar commits pendientes de subir
COMMITS_AHEAD=$(git rev-list --count "origin/${RAMA_ACTUAL}..HEAD" 2>/dev/null || echo "0")
echo -e "${COLOR_GRAY}> Commits locales listos para subir:${COLOR_RESET} ${COLOR_GREEN}${COMMITS_AHEAD}${COLOR_RESET}"

if [[ "$COMMITS_AHEAD" -gt 0 ]]; then
    echo -e "${COLOR_GRAY}> Últimos cambios que se enviarán a GitHub:${COLOR_RESET}"
    git log --oneline -n 5 "origin/${RAMA_ACTUAL}..HEAD" 2>/dev/null || true
fi

# Resumen previo
echo -e "\n${COLOR_CYAN}--- Resumen de Acciones ---${COLOR_RESET}"
echo -e "  * Se creará la etiqueta Git: ${COLOR_GREEN}${VERSION}${COLOR_RESET}"
echo -e "  * Se subirán ${COLOR_GREEN}${COMMITS_AHEAD}${COLOR_RESET} commits a la rama ${COLOR_CYAN}${RAMA_ACTUAL}${COLOR_RESET} en origin."
echo -e "  * Se enviará la etiqueta ${COLOR_GREEN}${VERSION}${COLOR_RESET} a origin."
echo -e "  * Se activará el flujo de GitHub Actions para generar los binarios .msi, .deb, .rpm y .dmg."

if [[ "$DRY_RUN" = true ]]; then
    echo -e "\n${COLOR_YELLOW}[Modo Dry-Run]: Verificación completada con éxito. No se han realizado cambios remotos.${COLOR_RESET}"
    exit 0
fi

# Confirmación interactiva
if [[ "$AUTO_CONFIRM" = false ]]; then
    echo -ne "\n${COLOR_YELLOW}¿Deseas continuar y publicar ${VERSION} en GitHub? [s/N]: ${COLOR_RESET}"
    read -r RESPUESTA
    if [[ ! "$RESPUESTA" =~ ^[sSyY]$ ]]; then
        echo -e "\n${COLOR_GRAY}Operación cancelada por el usuario. No se ha modificado nada.${COLOR_RESET}"
        exit 0
    fi
fi

# Ejecución del lanzamiento
echo -e "\n${COLOR_GRAY}> Subiendo rama ${RAMA_ACTUAL} a GitHub...${COLOR_RESET}"
git push origin "${RAMA_ACTUAL}"

echo -e "\n${COLOR_GRAY}> Creando etiqueta ${VERSION}...${COLOR_RESET}"
git tag -a "${VERSION}" -m "Lanzamiento oficial de LexiShield ${VERSION}"

echo -e "\n${COLOR_GRAY}> Subiendo etiqueta a GitHub...${COLOR_RESET}"
git push origin "${VERSION}"

echo -e "\n${COLOR_GREEN}========================================================${COLOR_RESET}"
echo -e "${COLOR_GREEN}  ¡Éxito! La versión ${VERSION} ha sido publicada.${COLOR_RESET}"
echo -e "${COLOR_GREEN}========================================================${COLOR_RESET}"
echo -e "GitHub Actions iniciará la compilación de paquetes en breve."
echo -e "Podrás seguir el progreso en la pestaña 'Actions' y 'Releases' de tu repositorio.\n"
