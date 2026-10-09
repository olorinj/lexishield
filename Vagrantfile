# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  # =========================================================================
  # 1. MAQUINA LINUX (Para Linux nativo y macOS cruzado via Docker)
  # =========================================================================
  config.vm.define "builder-linux" do |linux|
    linux.vm.box = "bento/debian-12"
    linux.vm.synced_folder ".", "/vagrant", mount_options: ["dmode=775", "fmode=775"]

    linux.vm.provider "virtualbox" do |vb|
      vb.name = "LINUX-DEV"
      vb.memory = "4096"
      vb.cpus = 4
      vb.customize ["modifyvm", :id, "--ioapic", "on"]
      vb.customize ["modifyvm", :id, "--paravirtprovider", "kvm"]
      vb.customize ["modifyvm", :id, "--nested-hw-virt", "on"]
      vb.customize ["modifyvm", :id, "--natdnshostresolver1", "on"]
      vb.customize ["modifyvm", :id, "--audio", "none"]
      vb.customize ["modifyvm", :id, "--vram", "16"]
      vb.customize ["modifyvm", :id, "--accelerate3d", "off"]
    end

    linux.vm.provision "shell", inline: <<-SHELL
      export DEBIAN_FRONTEND=noninteractive

      echo "=== Actualizando repositorios ==="
      apt-get update

      echo "=== Instalando dependencias de compilacion (Linux, Docker, GUI Tauri y utilidades de VirtualBox) ==="
      apt-get install -y --no-install-recommends \
        curl build-essential pkg-config libssl-dev git dos2unix docker.io ca-certificates \
        libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf file libfuse2 \
        virtualbox-guest-utils

      # Asegurar soporte de carpetas compartidas y punto de montaje
      modprobe vboxsf 2>/dev/null || true
      mkdir -p /vagrant
      if ! mountpoint -q /vagrant; then
        mount -t vboxsf -o uid=1000,gid=1000,dmode=775,fmode=775 vagrant /vagrant 2>/dev/null || true
      fi

      echo "=== Instalando Node.js (v20 LTS) y npm ==="
      curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
      apt-get install -y nodejs

      echo "=== Configurando servicio y permisos de Docker ==="
      usermod -aG docker vagrant
      systemctl enable --now docker

      echo "=== Instalando toolchain de Rust ==="
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
      su - vagrant -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default'

      /root/.cargo/bin/rustup target add x86_64-unknown-linux-gnu
      su - vagrant -c 'rustup target add x86_64-unknown-linux-gnu'

      # Exportar PATH globalmente para ejecucion remota via 'vagrant ssh'
      echo 'export PATH="/home/vagrant/.cargo/bin:/root/.cargo/bin:$PATH"' > /etc/profile.d/rust.sh
      chmod +x /etc/profile.d/rust.sh
      ln -sf /root/.cargo/bin/* /usr/local/bin/ || true
      ln -sf /home/vagrant/.cargo/bin/* /usr/local/bin/ || true

      echo "========================================================================="
      echo "¡Entorno de compilacion Linux/macOS listo!"
      echo "========================================================================="
    SHELL
  end

  # =========================================================================
  # 2. MAQUINA WINDOWS (Para CLI y GUI nativos con MSVC y Tauri)
  # =========================================================================
  config.vm.define "builder-windows", autostart: false do |win|
    win.vm.box = "gusztavvargadr/visual-studio-2022-community-windows-11"
    win.vm.boot_timeout = 600
    win.vm.communicator = "winrm"
    win.winrm.retry_limit = 60
    win.winrm.retry_delay = 10
    win.vm.synced_folder ".", "c:/vagrant"

    win.vm.provider "virtualbox" do |vb|
      vb.name = "WINDOWS-DEV"
      vb.memory = "8192"
      vb.cpus = 4
      vb.customize ["modifyvm", :id, "--ioapic", "on"]
      vb.customize ["modifyvm", :id, "--natdnshostresolver1", "on"]
      vb.customize ["modifyvm", :id, "--audio", "none"]
      vb.customize ["modifyvm", :id, "--vram", "128"]
      vb.customize ["modifyvm", :id, "--accelerate3d", "off"]
    end

    win.vm.provision "shell", privileged: false, inline: <<-POWERSHELL
      [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
      Write-Host "=== Verificando herramientas de compilacion en Windows ===" -ForegroundColor Cyan

      # Instalar Node.js LTS si no existe (usando Chocolatey)
      if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
          Write-Host "Instalando Node.js LTS..." -ForegroundColor Gray
          if (-not (Get-Command choco -ErrorAction SilentlyContinue)) {
              Write-Host "Instalando Chocolatey..." -ForegroundColor Gray
              Set-ExecutionPolicy Bypass -Scope Process -Force
              [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
              iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
              $env:PATH = "$env:ALLUSERSPROFILE\chocolatey\bin;$env:PATH"
          }
          choco install -y nodejs-lts
      }

      # Instalar Rustup y Toolchain MSVC si no existe
      if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
          Write-Host "Instalando Rust (MSVC Toolchain)..." -ForegroundColor Gray
          $rustupInit = "$env:TEMP\\rustup-init.exe"
          Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit
          Start-Process -FilePath $rustupInit -ArgumentList "-y", "--default-host", "x86_64-pc-windows-msvc", "--default-toolchain", "stable" -Wait -NoNewWindow
          Remove-Item -Force $rustupInit -ErrorAction SilentlyContinue
      }

      Write-Host "=========================================================================" -ForegroundColor Green
      Write-Host "¡Entorno de compilacion Windows listo!" -ForegroundColor Green
      Write-Host "=========================================================================" -ForegroundColor Green
    POWERSHELL

    win.vm.provision "shell", privileged: true, inline: <<-POWERSHELL
      Write-Host "=== Aplicando optimizaciones de sistema (Chris Titus Tech) ===" -ForegroundColor Cyan
      & ([ScriptBlock]::Create((irm https://christitus.com/win))) -Preset Standard

      Write-Host "=== Aplicando Windows 11 Debloat (Raphi) ===" -ForegroundColor Cyan
      & ([scriptblock]::Create((irm "https://debloat.raphi.re/"))) -RunDefaults -Silent

      Write-Host "=== Instalando MSVC C++ Build Tools (link.exe) ===" -ForegroundColor Cyan
      if (-not (Get-Command choco -ErrorAction SilentlyContinue)) {
          Set-ExecutionPolicy Bypass -Scope Process -Force
          [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
          iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))
          $env:PATH = "$env:ALLUSERSPROFILE\\chocolatey\\bin;$env:PATH"
      }
      choco install visualstudio2022buildtools -y --package-parameters "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --quiet --norestart"

      Write-Host "=== Desinstalando OneDrive ===" -ForegroundColor Cyan
      choco uninstall onedrive -y --force -ErrorAction SilentlyContinue
      Stop-Process -Name "OneDrive" -Force -ErrorAction SilentlyContinue
      $oneDriveUninstaller = "$env:SystemRoot\\SysWOW64\\OneDriveSetup.exe"
      if (Test-Path $oneDriveUninstaller) { Start-Process -FilePath $oneDriveUninstaller -ArgumentList "/uninstall" -Wait -NoNewWindow }

      Write-Host "=== Configurando teclado a Español ===" -ForegroundColor Cyan
      Set-WinUserLanguageList -LanguageList es-ES, en-US -Force

      Write-Host "=== Configurando Alto Rendimiento y Estética ===" -ForegroundColor Cyan
      powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
      
      Set-ItemProperty -Path "HKCU:\\Control Panel\\Colors" -Name "Background" -Value "0 0 0"
      Set-ItemProperty -Path "HKCU:\\Control Panel\\Desktop" -Name "Wallpaper" -Value ""
      RUNDLL32.EXE user32.dll,UpdatePerUserSystemParameters
      
      $Path = "HKLM:\\SOFTWARE\\Policies\\Microsoft\\Windows\\Personalization"
      if (-not (Test-Path $Path)) { New-Item -Path $Path -Force | Out-Null }
      Set-ItemProperty -Path $Path -Name "NoLockScreen" -Value 1 -Type DWord -Force
      
      Set-ItemProperty -Path "HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects" -Name "VisualFXSetting" -Value 2
    POWERSHELL
  end
end
