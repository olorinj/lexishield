[CmdletBinding()]
param(
    [Parameter(Position=0)]
    [string]$Version,

    [Alias("h")]
    [switch]$Help,

    [Alias("f")]
    [switch]$Force,

    [Alias("y")]
    [switch]$Yes,

    [Alias("d")]
    [switch]$DryRun
)

# Activar codificacion UTF-8 para consola en PowerShell
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

function Mostrar-Ayuda {
    Write-Host "`n================================================================================" -ForegroundColor Cyan
    Write-Host "  LexiShield - Asistente de Publicacion de Versiones en GitHub" -ForegroundColor Cyan
    Write-Host "================================================================================" -ForegroundColor Cyan
    Write-Host "`nEste script guia y automatiza el proceso de lanzamiento oficial en GitHub:"
    Write-Host "  1. Comprueba la rama activa y el estado del repositorio Git local."
    Write-Host "  2. Muestra un resumen claro de commits pendientes de subida y version a etiquetar."
    Write-Host "  3. Solicita confirmacion explicita antes de interactuar con el servidor remoto."
    Write-Host "  4. Sube la rama y la etiqueta a GitHub, activando el flujo automatizado de"
    Write-Host "     GitHub Actions para compilar los instaladores nativos (.msi, .deb, .rpm, .dmg)."
    Write-Host "`nUSO:" -ForegroundColor Cyan
    Write-Host "  .\publicar_version.ps1 [-Version] <VERSION> [-Force] [-DryRun] [-Yes] [-Help]"
    Write-Host "`nARGUMENTOS:" -ForegroundColor Cyan
    Write-Host "  -Version <STRING>   Version oficial a publicar (ej: v2.0.0 o 2.0.0)."
    Write-Host "`nOPCIONES:" -ForegroundColor Cyan
    Write-Host "  -Help, -h           Muestra esta pantalla de ayuda y sale."
    Write-Host "  -Force, -f          Fuerza la recreacion de la etiqueta si ya existe (local o remota)."
    Write-Host "  -Yes, -y            Modo desatendido: confirma automaticamente sin pausar."
    Write-Host "  -DryRun, -d         Modo simulacion: verifica el estado pero no realiza git push."
    Write-Host "`nEJEMPLOS:" -ForegroundColor Cyan
    Write-Host "  .\publicar_version.ps1 v2.0.0"
    Write-Host "  .\publicar_version.ps1 -Version 2.0.0"
    Write-Host "  .\publicar_version.ps1 -Version v2.0.0 -Force"
    Write-Host "  .\publicar_version.ps1 -Version v2.0.0 -DryRun`n"
}

if ($Help -or [string]::IsNullOrWhiteSpace($Version)) {
    Mostrar-Ayuda
    if ([string]::IsNullOrWhiteSpace($Version) -and -not $Help) {
        Write-Host "Por favor, especifica la version a publicar (ej: .\publicar_version.ps1 v2.0.0).`n" -ForegroundColor Yellow
        exit 1
    }
    exit 0
}

# Normalizar version
if (-not $Version.StartsWith("v")) {
    Write-Host "> Anadiendo prefijo "v" a la version: v$Version" -ForegroundColor Yellow
    $Version = "v" + $Version
}

Write-Host "`n========================================================" -ForegroundColor Cyan
Write-Host "    Lanzamiento de LexiShield $Version" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# Rama activa
$ramaActual = git branch --show-current 2>$null
if (-not $ramaActual) { $ramaActual = "desconocida" }
Write-Host "> Rama activa: " -NoNewline -ForegroundColor DarkGray
Write-Host $ramaActual -ForegroundColor Cyan

# Comprobar cambios sin confirmar
$gitStatus = git status -s
if (-not [string]::IsNullOrWhiteSpace($gitStatus)) {
    Write-Host "`n[!] Error: Tienes cambios locales pendientes de confirmacion en Git:" -ForegroundColor Red
    Write-Host $gitStatus
    Write-Host "`nPor seguridad, confirma o descarta tus cambios antes de publicar una version." -ForegroundColor Yellow
    exit 1
}

# Comprobar si el tag ya existe
git rev-parse "$Version" 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Host "`n[i] La etiqueta "$Version" ya existe en el repositorio local." -ForegroundColor Yellow
    $recrear = $false
    if ($Force) {
        $recrear = $true
    } else {
        $respTag = Read-Host "¿Deseas eliminar la version anterior (local y en GitHub) y recrearla con el commit actual? (s/N)"
        if ($respTag -match "^[sSyY]$") {
            $recrear = $true
        }
    }

    if ($recrear) {
        if ($DryRun) {
            Write-Host "[Dry-Run] Se eliminaria la etiqueta local y remota "$Version" para recrearla." -ForegroundColor DarkGray
        } else {
            Write-Host "> Eliminando etiqueta local "$Version"..." -ForegroundColor DarkGray
            git tag -d "$Version"
            Write-Host "> Eliminando etiqueta remota en GitHub (si existe)..." -ForegroundColor DarkGray
            git push origin --delete "$Version" 2>$null
            if (Get-Command "gh" -ErrorAction SilentlyContinue) {
                gh release delete "$Version" -y 2>$null
            }
        }
    } else {
        Write-Host "`nOperacion cancelada. La etiqueta "$Version" ya existe." -ForegroundColor Red
        Write-Host "Usa otra version o ejecuta .\publicar_version.ps1 -Version $Version -Force para sobrescribirla."
        exit 1
    }
}

# Commits pendientes
$commitsAhead = git rev-list --count "origin/$ramaActual..HEAD" 2>$null
if (-not $commitsAhead) { $commitsAhead = "0" }
Write-Host "> Commits locales listos para subir: " -NoNewline -ForegroundColor DarkGray
Write-Host $commitsAhead -ForegroundColor Green

if ([int]$commitsAhead -gt 0) {
    Write-Host "> Ultimos cambios que se enviaran a GitHub:" -ForegroundColor DarkGray
    git log --oneline -n 5 "origin/$ramaActual..HEAD" 2>$null
}

# Resumen previo
Write-Host "`n--- Resumen de Acciones ---" -ForegroundColor Cyan
Write-Host "  * Se creara la etiqueta Git: $Version" -ForegroundColor Green
Write-Host "  * Se subiran $commitsAhead commits a la rama $ramaActual en origin."
Write-Host "  * Se enviara la etiqueta $Version a origin."
Write-Host "  * Se activara el flujo de GitHub Actions para generar los binarios .msi, .deb, .rpm y .dmg."

if ($DryRun) {
    Write-Host "`n[Modo Dry-Run]: Verificacion completada con exito. No se han realizado cambios remotos." -ForegroundColor Yellow
    exit 0
}

# Confirmacion interactiva
if (-not $Yes) {
    $confirmacion = Read-Host "`n¿Deseas continuar y publicar $Version en GitHub? (s/N)"
    if ($confirmacion -notmatch "^[sSyY]$") {
        Write-Host "`nOperacion cancelada por el usuario. No se ha modificado nada." -ForegroundColor DarkGray
        exit 0
    }
}

Write-Host "`n> Subiendo rama $ramaActual a GitHub..." -ForegroundColor DarkGray
git push origin $ramaActual

Write-Host "`n> Creando etiqueta $Version..." -ForegroundColor DarkGray
git tag -a $Version -m "Lanzamiento oficial de LexiShield $Version"

Write-Host "`n> Subiendo etiqueta a GitHub..." -ForegroundColor DarkGray
git push origin $Version

Write-Host "`n========================================================" -ForegroundColor Green
Write-Host "  ¡Exito! La version $Version ha sido publicada." -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Green
Write-Host "GitHub Actions iniciara la compilacion de paquetes en breve."
Write-Host "Podras seguir el progreso en la pestana "Actions" y "Releases" de tu repositorio.`n"
