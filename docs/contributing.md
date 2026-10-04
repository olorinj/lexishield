# Guía de Contribución y Compilación

Este documento explica cómo configurar el entorno de desarrollo y compilar LexiShield para múltiples plataformas.

## 🛠️ Desarrollo Remoto y Compilación Cruzada Multiplataforma

Dada la exigencia de generar binarios nativos sin alertar a los EDR (antivirus) y la necesidad de compilación cruzada hacia Windows y macOS desde entornos aislados, **el único método oficial de compilación es mediante Vagrant**.

Se ha integrado un sistema de compilación automatizado y aislado que arranca una máquina virtual de Ubuntu, monta un disco ultrarrápido y realiza la generación de los tres binarios (`.exe` de Windows con metadatos incrustados, Linux ELF y macOS Mach-O). 

Para compilar el proyecto en todos los sistemas operativos simultáneamente de forma segura, solo tienes que ejecutar:

**En Linux / macOS:**
```bash
./compilar.sh --all
```

**En Windows:**
```powershell
.\compilar.ps1 -All
```

### Opciones de los Scripts de Compilación

Los scripts (`compilar.sh` y `compilar.ps1`) aceptan los siguientes argumentos para personalizar qué y cómo se compila en Vagrant:

| Opción Linux | Opción Windows | Descripción |
| :--- | :--- | :--- |
| `-g`, `--gui` | `-Gui` | Compila la interfaz gráfica (GUI) con Tauri y el frontend web. |
| `-a`, `--all` | `-All` | Realiza una compilación cruzada completa (todas las plataformas). |
| `-u`, `--provision` | `-Update` | **Fuerza la actualización de dependencias en Vagrant.** Útil si el `Vagrantfile` ha cambiado (ej. Node.js) y la VM ya estaba creada. |
| `-c`, `--clean` | `-Clean` | Destruye la máquina virtual (`vagrant destroy -f`) antes de empezar. |

## 🧪 Flujo de Calidad Obligatorio (Quality Gate)

Antes de enviar cualquier contribución o abrir una *Pull Request*, debes pasar el control de calidad en cuatro pasos:

1. **Formato:**
   ```bash
   cargo fmt --all -- --check
   ```
2. **Linting Pedante:**
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
3. **Pruebas Automatizadas:**
   ```bash
   cargo test --all-targets
   ```
4. **Auditoría de Seguridad de Dependencias:**
   ```bash
   cargo audit
   ```

## 🛡️ Pruebas de Fuzzing (`cargo-fuzz`)

Para verificar que los adaptadores y motores no producen pánicos ante datos corruptos:

```bash
# Ejecutar fuzzing del motor general
cargo +nightly fuzz run lexi_engine

# Fuzzing de adaptadores estructurados
cargo +nightly fuzz run fuzz_json
cargo +nightly fuzz run fuzz_xml
cargo +nightly fuzz run fuzz_log
```

## ⏱️ Ejecución de Benchmarks (`criterion`)

Para validar el rendimiento y *throughput* de escaneo y ofuscación:

```bash
cargo bench
```

## 🚀 Publicación y Generación de Instaladores (GitHub Actions)

El proyecto está configurado para compilar y empaquetar automáticamente las interfaces gráficas nativas (.msi, .dmg, .deb, .AppImage) en la nube utilizando **GitHub Actions**, dado que compilar la interfaz de Mac y Windows desde Linux local no es posible.

### 1. Vincular tu Repositorio a GitHub

Si tienes tu código local y todavía no lo has subido a GitHub (no tienes configurado el `remote`), debes seguir estos pasos primero:

1. Crea un repositorio vacío en la página web de [GitHub](https://github.com/new).
2. Abre tu terminal en la carpeta local de `lexishield`.
3. Ejecuta los siguientes comandos (cambiando la URL por la tuya):

```bash
# Vincular tu repositorio local con el de GitHub
git remote add origin https://github.com/TU_USUARIO/TU_REPOSITORIO.git

# Renombrar tu rama principal a 'main' (si no lo está ya)
git branch -M main

# Subir todo tu código inicial por primera vez
git push -u origin main
```

### 2. Generar una Nueva Versión (Release) Automática

Una vez tu código esté en GitHub, generar instaladores para tus usuarios es tan fácil como etiquetar una versión. Tienes unos scripts automáticos en la raíz del proyecto para hacer esto en un solo paso:

**En Linux / macOS:**
```bash
./publicar_version.sh v1.0.0
```

**En Windows (PowerShell):**
```powershell
.\publicar_version.ps1 v1.0.0
```

Estos scripts se asegurarán de que tu código esté limpio, crearán una etiqueta git (`v1.0.0`) y la subirán. Inmediatamente después, los servidores de GitHub Actions arrancarán y compilarán los instaladores gráficos para todos los sistemas operativos, depositándolos en la pestaña "Releases" (Lanzamientos) de tu repositorio.
