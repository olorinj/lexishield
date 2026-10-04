param(
    [Parameter(Mandatory=$true, HelpMessage="Indica la version (ej: v1.0.0)")]
    [string]$Version
)

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "    Publicar nueva version de LexiShield en GitHub" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

if (-not $Version.StartsWith("v")) {
    Write-Host "Aviso: Se recomienda que la version empiece por 'v'. Añadiendo 'v'..." -ForegroundColor Yellow
    $Version = "v" + $Version
}

Write-Host "`n> Comprobando estado de git..." -ForegroundColor DarkGray
$gitStatus = git status -s
if (-not [string]::IsNullOrWhiteSpace($gitStatus)) {
    Write-Host "Tienes cambios sin subir. Subiendolos primero..." -ForegroundColor Yellow
    git add .
    git commit -m "chore: preparar lanzamiento $Version"
    git push
} else {
    Write-Host "Todo limpio, continuando..."
}

Write-Host "`n> Creando etiqueta (Tag) local $Version..." -ForegroundColor DarkGray
git tag -a $Version -m "Lanzamiento oficial de LexiShield $Version"

Write-Host "`n> Subiendo etiqueta a GitHub..." -ForegroundColor DarkGray
git push origin $Version

Write-Host "`n¡Exito! La etiqueta $Version ha sido enviada a GitHub." -ForegroundColor Green
Write-Host "Esto disparara automaticamente el flujo de trabajo de GitHub Actions."
Write-Host "En unos 10-15 minutos, aparecera un borrador (Draft) en la pestaña 'Releases' de tu repositorio."
Write-Host "Contendra los instaladores para Windows (.msi), Mac (.dmg) y Linux (.AppImage, .deb)."

