#!/bin/bash
set -e

echo -e "\e[36m========================================================\e[0m"
echo -e "\e[36m    Publicar nueva versión de LexiShield en GitHub\e[0m"
echo -e "\e[36m========================================================\e[0m"

# Validar argumento
if [ -z "$1" ]; then
    echo -e "\e[31mError: Debes indicar la versión.\e[0m"
    echo "Uso: ./publicar_version.sh v1.0.0"
    exit 1
fi

VERSION=$1

# Comprobar que empieza por 'v'
if [[ ! $VERSION == v* ]]; then
    echo -e "\e[33mAviso: Se recomienda que la versión empiece por 'v' (ej: v1.0.0). Añadiendo 'v'...\e[0m"
    VERSION="v$VERSION"
fi

echo -e "\n\e[90m> Comprobando estado de git...\e[0m"
if [[ -n $(git status -s) ]]; then
    echo -e "\e[33mTienes cambios sin subir (uncommitted changes). Subiéndolos primero...\e[0m"
    git add .
    git commit -m "chore: preparar lanzamiento $VERSION"
    git push
else
    echo "Todo limpio, continuando..."
fi

echo -e "\n\e[90m> Creando etiqueta (Tag) local $VERSION...\e[0m"
git tag -a "$VERSION" -m "Lanzamiento oficial de LexiShield $VERSION"

echo -e "\n\e[90m> Subiendo etiqueta a GitHub...\e[0m"
git push origin "$VERSION"

echo -e "\n\e[32m¡Éxito! La etiqueta $VERSION ha sido enviada a GitHub.\e[0m"
echo -e "Esto disparará automáticamente el flujo de trabajo de GitHub Actions."
echo -e "En unos 10-15 minutos, aparecerá un borrador (Draft) en la pestaña 'Releases' de tu repositorio."
echo -e "Contendrá los instaladores para Windows (.msi), Mac (.dmg) y Linux (.AppImage, .deb)."

