#!/usr/bin/env bash
# ==============================================================================
# LexiShield - Script de Poda y Limpieza de Idiomas (Disk Footprint Optimizer)
# ==============================================================================
# Permite conservar únicamente los idiomas seleccionados en ui/src/locales/
# para reducir el tamaño del paquete web y optimizar espacio en disco.
#
# Uso:
#   ./scripts/prune_locales.sh en,es      # Mantiene inglés y español
#   ./scripts/prune_locales.sh en         # Mantiene exclusivamente inglés
# ==============================================================================

set -euo pipefail

TARGET_LANGS="${1:-}"

if [ -z "$TARGET_LANGS" ]; then
    echo "Uso: $0 <idiomas_separados_por_coma>"
    echo "Ejemplo: $0 en"
    echo "Ejemplo: $0 en,es"
    exit 1
fi

LOCALES_DIR="ui/src/locales"

if [ ! -d "$LOCALES_DIR" ]; then
    echo "Error: No se encontró el directorio $LOCALES_DIR"
    exit 1
fi

IFS=',' read -ra KEEP_ARRAY <<< "$TARGET_LANGS"

echo "Podando idiomas en $LOCALES_DIR para conservar únicamente: ${KEEP_ARRAY[*]}..."

for file in "$LOCALES_DIR"/*.json; do
    [ -e "$file" ] || continue
    filename=$(basename "$file" .json)
    
    keep=false
    for target in "${KEEP_ARRAY[@]}"; do
        if [ "$filename" = "$target" ]; then
            keep=true
            break
        fi
    done
    
    if [ "$keep" = false ]; then
        echo " -> Eliminando idioma no deseado: $file"
        rm -f "$file"
    else
        echo " -> Conservando idioma: $filename"
    fi
done

echo "Poda completada exitosamente."

