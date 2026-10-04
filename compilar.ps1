param (
    [Alias("w", "win")]
    [switch]$Windows,

    [Alias("l")]
    [switch]$Linux,

    [Alias("m", "macos")]
    [switch]$Mac,

    [Alias("a")]
    [switch]$All,

    [Alias("c")]
    [switch]$Clean,

    [Alias("u", "provision")]
    [switch]$Update,

    [Alias("k", "keep-vm")]
    [switch]$KeepVm,

    [Alias("native")]
    [switch]$Native
)

# Activar codificacion UTF-8 para consola en PowerShell
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$directorioOriginal = Get-Location
try {
    Push-Location -Path $PSScriptRoot

    # Modo Nativo: Si el usuario fuerza compilación local con herramientas instaladas en Windows
    if ($Native) {
        Write-Host "Ejecutando en modo nativo local (Windows)..." -ForegroundColor Cyan
        
        # Comprobar si link.exe (MSVC) esta disponible
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
            cargo clean
            if (Test-Path ".\target") {
                Remove-Item -Recurse -Force ".\target" -ErrorAction SilentlyContinue
            }
        }

        cargo clippy --release --fix --allow-dirty -- -D warnings
        cargo fmt
        cargo build --release

        if (Test-Path ".\target\release\lexishield.exe") {
            if (-not (Test-Path ".\dist")) { New-Item -ItemType Directory -Force -Path ".\dist" | Out-Null }
            Copy-Item -Path .\target\release\lexishield.exe -Destination .\dist\lexishield-windows-amd64.exe -Force
            Write-Host "Binario generado: .\dist\lexishield-windows-amd64.exe" -ForegroundColor Green
        }
        return
    }

    # Modo Centralizado (Por defecto): Usar Vagrant para compilar Linux, Windows y macOS sin dependencias locales
    if (-not (Get-Command "vagrant" -ErrorAction SilentlyContinue)) {
        Write-Host "`n[ERROR] Para compilar de forma aislada y multiplataforma se requiere 'vagrant'." -ForegroundColor Red
        Write-Host "Descargue e instale Vagrant (HashiCorp) y VirtualBox, o use el parametro -Native para compilar en local.`n" -ForegroundColor Yellow
        exit 1
    }

    # Construir lista de argumentos para pasar al script dentro de la VM
    $argsBash = @()
    if ($All) {
        $argsBash += "-a"
    } else {
        if ($Windows) { $argsBash += "-w" }
        if ($Linux)   { $argsBash += "-l" }
        if ($Mac)     { $argsBash += "-m" }
    }
    if ($Clean)  { $argsBash += "-c" }
    if ($Update) { $argsBash += "-u" }
    if ($KeepVm) { $argsBash += "-k" }

    $argumentosStr = $argsBash -join " "

    Write-Host "`n========================================================" -ForegroundColor Cyan
    Write-Host "Iniciando compilacion multiplataforma mediante Vagrant..." -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan

    if ($Update) {
        Write-Host "Iniciando y actualizando maquina virtual (Vagrant --provision)..." -ForegroundColor Gray
        vagrant up --provision
        if ($LASTEXITCODE -ne 0) {
            Write-Host "`n[ERROR] Fallo al aprovisionar la maquina virtual de Vagrant." -ForegroundColor Red
            exit $LASTEXITCODE
        }
    } else {
        Write-Host "Asegurando maquina virtual..." -ForegroundColor Gray
        vagrant up
        if ($LASTEXITCODE -ne 0) {
            Write-Host "`n[ERROR] Fallo al iniciar la maquina virtual de Vagrant." -ForegroundColor Red
            exit $LASTEXITCODE
        }
    }

    Write-Host "Ejecutando proceso de compilacion dentro de Vagrant..." -ForegroundColor Cyan
    vagrant ssh -c "cd /vagrant && dos2unix compilar.sh >/dev/null 2>&1 && bash ./compilar.sh $argumentosStr"
    $exitCodeBuild = $LASTEXITCODE

    # Apagar la máquina virtual si no se solicitó mantenerla encendida
    if (-not $KeepVm) {
        Write-Host "`nApagando maquina virtual para liberar recursos..." -ForegroundColor Gray
        vagrant halt
    }

    if ($exitCodeBuild -eq 0) {
        Write-Host "`nProceso completado con exito." -ForegroundColor Green
    } else {
        Write-Host "`n[ERROR] Ocurrio un error durante la compilacion en la maquina virtual." -ForegroundColor Red
    }
}
finally {
    Pop-Location
}
