# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  # Imagen minima y optimizada de Debian 12 (Bookworm) de Bento
  config.vm.box = "bento/debian-12"

  # Sincronizacion de la carpeta del proyecto en /vagrant
  config.vm.synced_folder ".", "/vagrant", mount_options: ["dmode=775", "fmode=775"]

  # Optimizacion de hardware y virtualizacion para maximo rendimiento
  config.vm.provider "virtualbox" do |vb|
    vb.name = "lexishield-builder"
    vb.memory = "4096"
    vb.cpus = 4

    # Ajustes avanzados de VirtualBox para aceleracion de compilacion
    vb.customize ["modifyvm", :id, "--ioapic", "on"]
    vb.customize ["modifyvm", :id, "--paravirtprovider", "kvm"]
    vb.customize ["modifyvm", :id, "--nested-hw-virt", "on"]
    vb.customize ["modifyvm", :id, "--natdnshostresolver1", "on"]
    vb.customize ["modifyvm", :id, "--audio", "none"]
    vb.customize ["modifyvm", :id, "--vram", "16"]
    vb.customize ["modifyvm", :id, "--accelerate3d", "off"]
  end

  # Aprovisionamiento automatizado de dependencias, MinGW, Docker y Rust
  config.vm.provision "shell", inline: <<-SHELL
    export DEBIAN_FRONTEND=noninteractive

    echo "=== Actualizando repositorios ==="
    apt-get update

    echo "=== Instalando dependencias de compilacion (Linux, MinGW para Windows, Docker y GUI Tauri) ==="
    apt-get install -y --no-install-recommends \
      curl build-essential pkg-config libssl-dev git dos2unix docker.io ca-certificates mingw-w64 \
      libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev

    echo "=== Instalando Node.js (v20 LTS) y npm ==="
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs

    echo "=== Configurando servicio y permisos de Docker ==="
    usermod -aG docker vagrant
    systemctl enable --now docker

    echo "=== Instalando toolchain de Rust y targets multiplataforma ==="
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default
    su - vagrant -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default'

    # Anadir targets oficiales de Linux y Windows (MinGW)
    /root/.cargo/bin/rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
    su - vagrant -c 'rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu'

    # Exportar PATH globalmente para ejecucion remota via 'vagrant ssh -c'
    echo 'export PATH="/home/vagrant/.cargo/bin:/root/.cargo/bin:$PATH"' > /etc/profile.d/rust.sh
    chmod +x /etc/profile.d/rust.sh
    ln -sf /root/.cargo/bin/* /usr/local/bin/ || true
    ln -sf /home/vagrant/.cargo/bin/* /usr/local/bin/ || true

    echo ""
    echo "========================================================================="
    echo "¡Entorno de compilacion LexiShield (Win/Linux/Mac) listo para su uso!"
    echo "========================================================================="
  SHELL
end
