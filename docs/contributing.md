# Guía de contribución y compilación

Este documento explica cómo configurar el entorno de desarrollo y compilar LexiShield para múltiples plataformas.

## Compilación y empaquetado oficial (`dev-forge`)

LexiShield delega todo su ciclo de compilación, construcción nativa y empaquetado multiplataforma a la herramienta universal **[`dev-forge`](../../dev-forge)**.

Todas las compilaciones (tanto locales como aisladas en máquinas virtuales para evadir alertas de antivirus/EDRs) se gestionan a través de `dev-forge` desde la raíz del proyecto:

### 1. Compilación rápida en el sistema actual (modo nativo)
Para compilar los binarios de la CLI directamente con el toolchain de Rust del anfitrión:
```bash
dev-forge build --native
```

### 2. Compilación multiplataforma aislada (Linux, Windows y macOS)
Para compilar de forma aislada y reproducible para todas las plataformas soportadas sin ensuciar el sistema anfitrión:
```bash
dev-forge build --all
```

### 3. Compilación por plataforma específica
```bash
# Compilar únicamente para Linux (x86_64)
dev-forge build --linux

# Compilar para Windows (x86_64 con MSVC en entorno aislado)
dev-forge build --windows

# Compilar para macOS (Intel x86_64 y Apple Silicon ARM64 con Docker/osxcross)
dev-forge build --mac
```

### 4. Compilación de la interfaz gráfica (desktop Tauri GUI)
Para compilar la aplicación de escritorio nativa (.deb, .rpm, .msi, etc.):
```bash
dev-forge build --gui
```

Los artefactos finales compilados se depositan automáticamente en la carpeta `dist/` en la raíz del proyecto.

## Flujo de calidad obligatorio (quality gate)

Antes de enviar cualquier contribución o abrir una *Pull Request*, debes pasar el control de calidad en cuatro pasos:

1. **Formato:**
   ```bash
   cargo fmt --all -- --check
   ```
2. **Linting pedante:**
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
3. **Pruebas automatizadas:**
   ```bash
   cargo test --all-targets
   ```
4. **Auditoría de seguridad de dependencias:**
   ```bash
   cargo audit
   ```

## Pruebas de fuzzing (`cargo-fuzz`)

Para verificar que los adaptadores y motores no producen pánicos ante datos corruptos:

```bash
# Ejecutar fuzzing del motor general
cargo +nightly fuzz run lexi_engine

# Fuzzing de adaptadores estructurados
cargo +nightly fuzz run fuzz_json
cargo +nightly fuzz run fuzz_xml
cargo +nightly fuzz run fuzz_log
```

## Ejecución de benchmarks (`criterion`)

Para validar el rendimiento y *throughput* de escaneo y ofuscación:

```bash
cargo bench
```

## Publicación y generación de instaladores (GitHub Actions)

El proyecto está configurado para compilar y empaquetar automáticamente las interfaces gráficas nativas (.msi, .dmg, .deb, .AppImage) en la nube utilizando **GitHub Actions**, dado que compilar la interfaz de Mac y Windows desde Linux local no es posible.

### 1. Vincular tu repositorio a GitHub

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

### 2. Generar una nueva versión (release) automática

Una vez tu código esté en GitHub, generar instaladores para tus usuarios es tan fácil como etiquetar una versión. Tienes unos scripts automáticos en la raíz del proyecto para hacer esto en un solo paso:

**En Linux / macOS:**
```bash
./publicar_version.sh v2.2.0
```

**En Windows (PowerShell):**
```powershell
.\publicar_version.ps1 v2.2.0
```

Estos scripts se asegurarán de que tu código esté limpio, crearán una etiqueta git (`v2.2.0`) y la subirán. Inmediatamente después, los servidores de GitHub Actions arrancarán y compilarán los instaladores gráficos para todos los sistemas operativos, depositándolos en la pestaña "Releases" (Lanzamientos) de tu repositorio.

---

## 3. Optimización de compilación y microarquitectura

Por diseño y para garantizar la distribución universal, los binarios oficiales se compilan para arquitecturas base estándar (`x86_64` genérica y `aarch64`), asegurando que cualquier usuario pueda ejecutarlos sin importar la antigüedad de su procesador.

### 3.1 Compilación optimizada para tu CPU local (`target-cpu=native`)
Si estás compilando LexiShield desde el código fuente para utilizarlo en tu propia máquina (o en un servidor dedicado) y deseas exprimir al 100% las instrucciones vectoriales nativas de tu CPU (AVX2, AVX-512, FMA, BMI2, NEON):

```bash
# Compilar binario de producción optimizado específicamente para tu procesador:
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Ejecutar la suite de pruebas de rendimiento sobre tu hardware:
RUSTFLAGS="-C target-cpu=native" cargo bench --bench engine_benchmark
```

### 3.2 Niveles de microarquitectura intermedia (`x86-64-v2` / `x86-64-v3`)
Para despliegues en flotas corporativas homogéneas donde se desea mayor rendimiento que la línea base pero sin atarse a un procesador individual:
* **`x86-64-v2`**: Compatible con casi cualquier CPU fabricada desde 2009 (soporta SSE4.2, SSSE3 y PopCnt).
  ```bash
  RUSTFLAGS="-C target-cpu=x86-64-v2" cargo build --release
  ```
* **`x86-64-v3`**: Optimizado para procesadores desde 2015 en adelante (activa AVX, AVX2 y BMI2).
  ```bash
  RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release
  ```
