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
        Write-Host "Buscando una instalacion valida de MSVC en el disco..." -ForegroundColor Gray
        $vcvarsFiles = Get-ChildItem -Path "${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio" -Filter "vcvars64.bat" -Recurse -ErrorAction SilentlyContinue
        
        $validVcvars = $null
        foreach ($vcvars in $vcvarsFiles) {
            # La ruta de vcvars64.bat es típicamente: <VS_PATH>\VC\Auxiliary\Build\vcvars64.bat
            # Las herramientas del compilador residen en: <VS_PATH>\VC\Tools\MSVC\
            $vsPath = $vcvars.Directory.Parent.Parent.Parent.FullName
            if ($vsPath -match "Community") {
                continue
            }
            $linkerFound = Get-ChildItem -Path "$vsPath\VC\Tools\MSVC" -Filter "link.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($linkerFound) {
                $validVcvars = $vcvars
                break
            }
        }

        if (-not $validVcvars) {
            Write-Host "AVISO: No se encontro el compilador de C++ (VC\Tools\MSVC). Instalando directamente..." -ForegroundColor Yellow
            $vsInstaller = "$env:TEMP\vs_buildtools.exe"
            Invoke-WebRequest -Uri "https://aka.ms/vs/17/release/vs_buildtools.exe" -OutFile $vsInstaller
            Start-Process -FilePath $vsInstaller -ArgumentList "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --quiet --wait --norestart --nocache" -Wait -NoNewWindow
            Remove-Item -Force $vsInstaller -ErrorAction SilentlyContinue
            
            $vcvarsFiles = Get-ChildItem -Path "${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio" -Filter "vcvars64.bat" -Recurse -ErrorAction SilentlyContinue
            foreach ($vcvars in $vcvarsFiles) {
                $vsPath = $vcvars.Directory.Parent.Parent.Parent.FullName
                if ($vsPath -match "Community") {
                    continue
                }
                $linkerFound = Get-ChildItem -Path "$vsPath\VC\Tools\MSVC" -Filter "link.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($linkerFound) {
                    $validVcvars = $vcvars
                    break
                }
            }
        }

        if ($validVcvars) {
            Write-Host "Inicializando entorno de compilacion MSVC desde $($validVcvars.FullName)..." -ForegroundColor Gray
            
            $tempBat = "$env:TEMP\init_msvc.bat"
            "call `"$($validVcvars.FullName)`" > nul" | Out-File -FilePath $tempBat -Encoding ascii
            "set" | Out-File -FilePath $tempBat -Append -Encoding ascii
            
            $envLines = cmd.exe /c $tempBat
            foreach ($line in $envLines) {
                if ($line -match "^(.*?)=(.*)$") {
                    Set-Item -Path "env:\$($matches[1])" -Value $matches[2]
                }
            }
            Remove-Item -Force $tempBat -ErrorAction SilentlyContinue
        } else {
            Write-Host "ERROR: No se pudo instalar ni localizar MSVC Build Tools. La compilacion de Rust fallara." -ForegroundColor Red
        }
    }

    # Forzar reinstalacion del toolchain para restaurar archivos .rlib eliminados previamente por Windows Defender
    # y configurar CARGO_BUILD_JOBS para evitar que cargo consuma toda la RAM (4GB) y crashee WinRM (WSMAN 1726).
    if (Get-Command "rustup" -ErrorAction SilentlyContinue) {
        Write-Host "Reparando integridad del toolchain de Rust y limitando concurrencia de memoria..." -ForegroundColor Yellow
        rustup toolchain install stable-x86_64-pc-windows-msvc --force 2>&1 | Out-Null
        rustup default stable-x86_64-pc-windows-msvc 2>&1 | Out-Null
        $env:CARGO_BUILD_JOBS = "1"
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

