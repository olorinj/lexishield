param (
    [Alias("w", "win")]
    [switch]$Windows,

    [Alias("l")]
    [switch]$Linux,

    [Alias("m", "macos")]
    [switch]$Mac,

    [Alias("g", "gui")]
    [switch]$GuiVersion,

    [Alias("a")]
    [switch]$All,

    [Alias("c")]
    [switch]$Clean,

    [Alias("u", "provision")]
    [switch]$Update,

    [Alias("k", "keep-vm")]
    [switch]$KeepVm,

    [Alias("native")]
    [switch]$NativeHost
)

# Activar codificacion UTF-8 para consola en PowerShell
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$directorioOriginal = Get-Location
try {
    Push-Location -Path $PSScriptRoot

    # Modo Nativo: Si el usuario fuerza compilación local con herramientas instaladas en Windows
    if ($NativeHost) {
        Write-Host "Ejecutando en modo nativo local (Windows)..." -ForegroundColor Cyan
        $argsWin = @()
        if ($GuiVersion) { $argsWin += "-g" }
        if ($All)        { $argsWin += "-a" }
        if ($Clean)      { $argsWin += "-c" }
        & "$PSScriptRoot\compilar-win.ps1" @argsWin
        return
    }

    # Modo Centralizado (Por defecto): Usar Vagrant para compilar Linux, Windows y macOS sin dependencias locales
    if (-not (Get-Command "vagrant" -ErrorAction SilentlyContinue)) {
        Write-Host "`n[ERROR] Para compilar de forma aislada y multiplataforma se requiere 'vagrant'." -ForegroundColor Red
        Write-Host "Descargue e instale Vagrant (HashiCorp) y VirtualBox, o use el parametro -Native para compilar en local.`n" -ForegroundColor Yellow
        exit 1
    }

    # Determinar qué máquinas virtuales se requieren
    $needLinux = $false
    $needWindows = $false

    if ($All) {
        $needLinux = $true
        $needWindows = $true
    } else {
        if ($Windows) { $needWindows = $true }
        if ($Linux -or $Mac) { $needLinux = $true }
        # Si solo se pasa -g sin plataforma, se compila en Linux por defecto a menos que se especifique Windows
        if (-not $Windows -and -not $Linux -and -not $Mac) {
            $needLinux = $true
        }
    }

    Write-Host "`n========================================================" -ForegroundColor Cyan
    Write-Host "Iniciando orquestacion inteligente con Vagrant..." -ForegroundColor Cyan
    Write-Host "  -> Requiere builder-linux:   $needLinux" -ForegroundColor Gray
    Write-Host "  -> Requiere builder-windows: $needWindows" -ForegroundColor Gray
    Write-Host "========================================================" -ForegroundColor Cyan

    $exitCodeBuild = 0

    # 1. Compilación en builder-linux (Linux nativo y macOS con Docker)
    if ($needLinux) {
        Write-Host "`n[Fase 1] Preparando VM 'builder-linux'..." -ForegroundColor Cyan
        $argsLinux = @()
        if ($Linux)      { $argsLinux += "-l" }
        if ($Mac)        { $argsLinux += "-m" }
        if ($GuiVersion) { $argsLinux += "-g" }
        if ($All)        { $argsLinux += "-a" }
        if ($Clean)      { $argsLinux += "-c" }
        if ($Update)     { $argsLinux += "-u" }
        if ($KeepVm)     { $argsLinux += "-k" }
        $argsLinuxStr = $argsLinux -join " "

        if ($Update) {
            Write-Host "Iniciando y aprovisionando builder-linux..." -ForegroundColor Gray
            vagrant up builder-linux --provision
        } else {
            Write-Host "Iniciando builder-linux..." -ForegroundColor Gray
            vagrant up builder-linux
        }

        if ($LASTEXITCODE -ne 0) {
            Write-Host "`n[ERROR] Fallo al iniciar builder-linux." -ForegroundColor Red
            exit $LASTEXITCODE
        }

        Write-Host "Ejecutando proceso de compilacion dentro de builder-linux..." -ForegroundColor Cyan
        vagrant ssh builder-linux -c "cd /vagrant && dos2unix compilar.sh >/dev/null 2>&1 && bash ./compilar.sh $argsLinuxStr"
        $exitCodeBuild = $LASTEXITCODE

        # Apagar builder-linux para liberar memoria y CPU antes de la siguiente fase
        if (-not $KeepVm) {
            Write-Host "`nLiberando recursos: Apagando builder-linux..." -ForegroundColor Gray
            vagrant halt builder-linux
        }

        if ($exitCodeBuild -ne 0) {
            Write-Host "`n[ERROR] La compilacion en builder-linux ha fallado." -ForegroundColor Red
            exit $exitCodeBuild
        }
    }

    # 2. Compilación en builder-windows (CLI MSVC y GUI Tauri nativos)
    if ($needWindows) {
        Write-Host "`n[Fase 2] Preparando VM 'builder-windows'..." -ForegroundColor Cyan
        $argsWin = @()
        if ($GuiVersion) { $argsWin += "-g" }
        if ($All)        { $argsWin += "-a" }
        if ($Clean)      { $argsWin += "-c" }
        $argsWinStr = $argsWin -join " "

        if ($Update) {
            Write-Host "Iniciando y aprovisionando builder-windows..." -ForegroundColor Gray
            vagrant up builder-windows --provision
        } else {
            Write-Host "Iniciando builder-windows..." -ForegroundColor Gray
            vagrant up builder-windows
        }

        if ($LASTEXITCODE -ne 0) {
            Write-Host "`n[ERROR] Fallo al iniciar builder-windows." -ForegroundColor Red
            exit $LASTEXITCODE
        }

        Write-Host "Ejecutando proceso de compilacion dentro de builder-windows..." -ForegroundColor Cyan
        vagrant winrm builder-windows -c "powershell -ExecutionPolicy Bypass -File C:\vagrant\compilar-win.ps1 $argsWinStr"
        if ($LASTEXITCODE -ne 0) {
            # Intentar via ssh si winrm no responde
            vagrant ssh builder-windows -c "powershell -ExecutionPolicy Bypass -File C:\vagrant\compilar-win.ps1 $argsWinStr"
        }
        $exitCodeBuild = $LASTEXITCODE

        # Apagar builder-windows para liberar memoria y CPU
        if (-not $KeepVm) {
            Write-Host "`nLiberando recursos: Apagando builder-windows..." -ForegroundColor Gray
            vagrant halt builder-windows
        }

        if ($exitCodeBuild -ne 0) {
            Write-Host "`n[ERROR] La compilacion en builder-windows ha fallado." -ForegroundColor Red
            exit $exitCodeBuild
        }
    }

    if ($exitCodeBuild -eq 0) {
        Write-Host "`n========================================================" -ForegroundColor Green
        Write-Host "Proceso de compilacion completado con exito." -ForegroundColor Green
        Write-Host "========================================================" -ForegroundColor Green
    }
}
finally {
    Pop-Location
}
