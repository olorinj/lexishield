param (
    [Alias("g", "gui")]
    [switch]$GuiVersion,

    [Alias("a")]
    [switch]$All,

    [Alias("c")]
    [switch]$Clean,

    [Alias("w", "win")]
    [switch]$Windows
)

# Activar codificacion UTF-8 para consola en PowerShell
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$directorioOriginal = Get-Location
try {
    Push-Location -Path $PSScriptRoot

    Write-Host "`n========================================================" -ForegroundColor Cyan
    Write-Host "Iniciando compilacion nativa para Windows (MSVC)..." -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan

    # Asegurar vcvars64 si link.exe no esta en el PATH
    if (-not (Get-Command "link.exe" -ErrorAction SilentlyContinue)) {
        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $vswhere) {
            $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
            if ($vsPath -and (Test-Path "$vsPath\VC\Auxiliary\Build\vcvars64.bat")) {
                Write-Host "Inicializando entorno de compilacion MSVC..." -ForegroundColor Gray
                $envLines = cmd.exe /c "call `"$vsPath\VC\Auxiliary\Build\vcvars64.bat`" > nul && set"
                foreach ($line in $envLines) {
                    if ($line -match "^(.*?)=(.*)$") {
                        Set-Item -Path "env:\$($matches[1])" -Value $matches[2]
                    }
                }
            }
        }
    }

    if ($Clean) {
        Write-Host "Limpiando artefactos previos de compilacion..." -ForegroundColor Yellow
        cargo clean
        if (Test-Path ".\target") { Remove-Item -Recurse -Force ".\target" -ErrorAction SilentlyContinue }
        if (Test-Path ".\src-tauri\target") { Remove-Item -Recurse -Force ".\src-tauri\target" -ErrorAction SilentlyContinue }
    }

    # Crear carpeta dist si no existe
    if (-not (Test-Path ".\dist")) {
        New-Item -ItemType Directory -Force -Path ".\dist" | Out-Null
    }

    # 1. Compilar CLI de Windows con MSVC
    Write-Host "`n[1/2] Compilando CLI de LexiShield (Release x86_64-pc-windows-msvc)..." -ForegroundColor Cyan
    cargo clippy --release --fix --allow-dirty -- -D warnings
    cargo fmt
    cargo build --release

    $archivosGenerados = @()

    if (Test-Path ".\target\release\lexishield.exe") {
        Copy-Item -Path ".\target\release\lexishield.exe" -Destination ".\dist\lexishield-windows-amd64.exe" -Force
        $archivosGenerados += ".\dist\lexishield-windows-amd64.exe"
        Write-Host "  -> Binario CLI generado: .\dist\lexishield-windows-amd64.exe" -ForegroundColor Green
    } else {
        Write-Host "[ERROR] No se pudo encontrar el binario generado de la CLI." -ForegroundColor Red
        exit 1
    }

    # 2. Compilar GUI con Tauri si se solicita (-g o -a)
    if ($GuiVersion -or $All) {
        Write-Host "`n[2/2] Compilando GUI de LexiShield con Tauri..." -ForegroundColor Cyan
        if (Get-Command "npm" -ErrorAction SilentlyContinue) {
            Write-Host "Instalando dependencias frontend (npm install)..." -ForegroundColor Gray
            Push-Location .\ui
            npm install
            npm run build
            Pop-Location

            $env:NO_STRIP = "true"
            Write-Host "Construyendo ejecutable nativo e instaladores con Tauri..." -ForegroundColor Gray
            if (Test-Path ".\ui\node_modules\.bin\tauri.cmd") {
                & .\ui\node_modules\.bin\tauri.cmd build
            } else {
                npx tauri build
            }

            if (Test-Path ".\src-tauri\target\release\lexishield-gui.exe") {
                Copy-Item -Path ".\src-tauri\target\release\lexishield-gui.exe" -Destination ".\dist\lexishield-gui-windows-amd64.exe" -Force
                $archivosGenerados += ".\dist\lexishield-gui-windows-amd64.exe"
                Write-Host "  -> Binario GUI generado: .\dist\lexishield-gui-windows-amd64.exe" -ForegroundColor Green
            }

            # Copiar instaladores MSI
            $msiFiles = Get-ChildItem -Path ".\src-tauri\target\release\bundle\msi\*.msi" -ErrorAction SilentlyContinue
            foreach ($msi in $msiFiles) {
                Copy-Item -Path $msi.FullName -Destination ".\dist\" -Force
                $archivosGenerados += ".\dist\$($msi.Name)"
                Write-Host "  -> Instalador MSI generado: .\dist\$($msi.Name)" -ForegroundColor Green
            }

            # Copiar instaladores NSIS
            $nsisFiles = Get-ChildItem -Path ".\src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue
            foreach ($nsis in $nsisFiles) {
                Copy-Item -Path $nsis.FullName -Destination ".\dist\" -Force
                $archivosGenerados += ".\dist\$($nsis.Name)"
                Write-Host "  -> Instalador NSIS generado: .\dist\$($nsis.Name)" -ForegroundColor Green
            }
        } else {
            Write-Host "[AVISO] 'npm' no encontrado en el PATH. Omitiendo compilacion de GUI." -ForegroundColor Yellow
        }
    }

    Write-Host "`n========================================================" -ForegroundColor Green
    Write-Host "Compilacion de Windows finalizada exitosamente." -ForegroundColor Green
    Write-Host "========================================================" -ForegroundColor Green
}
finally {
    Pop-Location
}

