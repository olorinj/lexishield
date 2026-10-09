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

    # Asegurar que Cargo y Node.js esten en el PATH de esta sesion
    $cargoPath = "$env:USERPROFILE\.cargo\bin"
    if ($env:PATH -notlike "*$cargoPath*") {
        $env:PATH = "$cargoPath;$env:PATH"
    }
    $nodePath = "$env:ProgramFiles\nodejs"
    if ($env:PATH -notlike "*$nodePath*") {
        $env:PATH = "$nodePath;$env:PATH"
    }

    Write-Host "`n========================================================" -ForegroundColor Cyan
    Write-Host "Iniciando compilacion nativa para Windows (MSVC)..." -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan

    # Asegurar vcvars64 si link.exe no esta en el PATH
    if (-not (Get-Command "link.exe" -ErrorAction SilentlyContinue)) {
        Write-Host "Buscando vcvars64.bat..." -ForegroundColor Gray
        $vcvars = Get-ChildItem -Path "${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio" -Filter "vcvars64.bat" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($vcvars) {
            Write-Host "Inicializando entorno de compilacion MSVC desde $($vcvars.FullName)..." -ForegroundColor Gray
            $envLines = cmd.exe /c "call `"$($vcvars.FullName)`" > nul && set"
            foreach ($line in $envLines) {
                if ($line -match "^(.*?)=(.*)$") {
                    Set-Item -Path "env:\$($matches[1])" -Value $matches[2]
                }
            }
        } else {
            Write-Host "ERROR: No se encontro vcvars64.bat en el sistema. Asegurate de que MSVC Build Tools se instalo." -ForegroundColor Red
        }
    }

    # Asegurar que haya un toolchain por defecto (corrige error "no default is configured")
    if (Get-Command "rustup" -ErrorAction SilentlyContinue) {
        $active = rustup show active-toolchain 2>&1
        if ("$active" -match "no active toolchain" -or "$active" -match "no default is configured") {
            Write-Host "Configurando toolchain de Rust (stable) por defecto..." -ForegroundColor Yellow
            rustup default stable
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

