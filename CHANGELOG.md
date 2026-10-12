# Changelog

Todos los cambios notables de este proyecto se documentarán en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-10-12
*Lanzamiento oficial de LexiShield 2.0.0: Interfaz gráfica de escritorio nativa (Tauri v2 + Svelte), arquitectura Zero-Knowledge y distribución multiplataforma unificada (CLI + GUI).*

### Añadido
- **🖥️ LexiShield GUI (Tauri v2 + Svelte 4 + TailwindCSS)**: Aplicación de escritorio nativa multiplataforma (Linux, Windows, macOS) para anonimización visual, auditoría y gestión de tokens sensibles.
- **✨ Studio para Inteligencia Artificial y Documentos**:
  - Sanitización contextual con preservación de formato estructurado (`JSON`, `XML`, `Logs`, `Markdown`, `Texto plano`).
  - Integración nativa con Pandoc para conversión a Markdown estructurado (.md), optimizando el consumo de tokens en LLMs.
  - Flujo de desofuscación biyectiva 1:1 para restaurar respuestas generadas por cualquier modelo de IA (ChatGPT, Claude, Gemini, IAs locales).
- **🔍 Escáner Preventivo de Fugas**: Auditoría pasiva de archivos individuales o directorios masivos con respeto nativo a `.gitignore` y `.lexiignore`, y telemetría de avance en vivo.
- **📑 Pares de Tokens Biyectivos**: Gestión interactiva del diccionario de correspondencias, soporte para añadir pares personalizados, marcar tokens a ignorar (`=`) y restauración rápida desde copias de seguridad `.bak`.
- **🗄️ Bóvedas Lexi y Copias Bzip2**: Contenedores cifrados `.lexi` protegidos con contraseña maestra (*Argon2id + ChaCha20-Poly1305*), con creador de copias de seguridad comprimidas en Bzip2 y opcionalmente cifradas en AES-256.
- **📋 Portapapeles Inteligente (Watch Mode)**:
  - Monitorización reactiva con transformación automática en segundo plano (`Ctrl+C` / `Cmd+C`).
  - Notificación explícita de *"0 tokens confidenciales detectados"* cuando el texto no contiene secretos.
  - Vista previa comparativa en tiempo real entre el texto capturado y el resultado transformado.
  - Vaciado seguro manual o automático temporizado para evitar persistencia de secretos en memoria del sistema operativo.
- **📘 Sistema de Ayuda Contextual y Onboarding Criptográfico**:
  - Botones dedicados `❔ Ayuda` en todas las pestañas con manual interactivo navegable.
  - Tarjetas de primer uso explicativas y descartables con memoria local (`localStorage`).
  - Diálogo formativo sobre la necesidad de contraseña obligatoria bajo el principio de Cero Conocimiento (*Zero-Knowledge Architecture*).
- **🎨 Accesibilidad y Ergonomía Visual**:
  - Soporte completo de Modo Claro y Modo Oscuro con paleta de alto contraste.
  - Indicadores visuales y accesibilidad W3C en componentes y modales (0 advertencias en compilación).
  - Registro de iconos nativos en Linux (Wayland y X11 vía `app_id`), Windows y macOS.
- **🚀 Publicación Asistida Multiplataforma**: Scripts [`publicar_version.sh`](file:///home/olorinj/DEV/rust/lexishield/publicar_version.sh) y [`publicar_version.ps1`](file:///home/olorinj/DEV/rust/lexishield/publicar_version.ps1) con menú `--help`, modo simulación `--dry-run`, validaciones de Git y confirmación interactiva.

### Mejorado
- **📦 Distribución CI/CD en GitHub Releases**:
  - El flujo de GitHub Actions compila y adjunta a la vez tanto los paquetes de la GUI (`.msi`, `.deb`, `.rpm`, `.dmg`) como los binarios de consola CLI independientes (`lexishield-windows-amd64.exe`, `lexishield-linux-amd64`, `lexishield-darwin-arm64/amd64`).
- **🔗 Portabilidad de Compilación (bzip2 Estático)**: Activada la característica `static` en la dependencia `bzip2` de `Cargo.toml`, garantizando enlace estático en C sin depender de `libbz2` dinámico en el sistema host.
- **🧹 Higiene del Repositorio y Exclusiones**: Ampliado [`.gitignore`](file:///home/olorinj/DEV/rust/lexishield/.gitignore) con exclusiones preventivas para artefactos de frontend (`ui/dist/`), temporales de editores (`*.bak`, `*.tmp`), metadatos de SO y bóvedas de prueba.

### Arreglado
- **Colisión de Recursos en Windows (MSVC)**: Resuelto el error `CVTRES : fatal error CVT1100: duplicate resource` en `link.exe` mediante la desconexión selectiva de `winres` (`DISABLE_WINRES` / `SKIP_WIN_RESOURCE`) durante las compilaciones de Tauri.
- **Aislamiento en Vagrant**: Optimizado [`compilar.sh`](file:///home/olorinj/DEV/rust/lexishield/compilar.sh) para aislar la compilación de frontend en `/tmp`, evitando cuellos de botella de I/O en carpetas compartidas `vboxsf`.

## [2.0.0-alpha.2] - 2026-10-05
*Hito de motor documental Office OpenXML y consolidación de infraestructura.*

### Añadido
- **Soporte para Ficheros Office**: Nuevo adaptador especializado (`office_adapter.rs`) para procesar `.docx`, `.xlsx` y `.pptx`. Habilita la lectura del contenedor, ofuscación y reempaquetado sin alterar el esquema XML (mapeo biyectivo estricto).
- **Especificaciones Técnicas**: Creación del documento base `docs/specs.md` para trazar la ruta arquitectónica de la versión final.

### Mejorado
- **Pipeline de CI/CD**: Actualización de la infraestructura de compilación continua (GitHub Actions) escalando a Node.js 22 para garantizar compatibilidad a largo plazo de los runners y evitar avisos de obsolescencia.

### Arreglado
- **Integridad del Repositorio**: Purgada la colisión de dependencias (`ui/node_modules`) del control de versiones que bloqueaba la compilación limpia del empaquetador Vite en integración continua.

## [2.0.0-alpha.1] - 2026-10-05
*Hito de modernización de Interfaz Gráfica (GUI) y empaquetado de escritorio.*

### Mejorado
- **Experiencia de Usuario (UI/UX)**: Rediseño completo del frontend (Tauri + Svelte + Tailwind) orientándolo hacia una arquitectura moderna "bento-box", con colores corporativos (cyan/navy).
- **Usabilidad de Escritorio**: Inyección nativa de áreas de arrastre (`data-tauri-drag-region`) en la barra de ventana de la aplicación y transiciones fluidas de estado interactivo.

### Arreglado
- **Empaquetado en Linux (AppImage)**: Corrección crítica del escalado de iconos (generación matemática y automatizada de 32x32 hasta 512x512) previniendo los bloqueos del *bundler* en Tauri.
- **Vagrant FUSE**: Parche de compatibilidad introducido para ejecutar `linuxdeploy` de forma desatendida dentro de contenedores automatizados Vagrant sin requerir FUSE local.

## [1.0.0] - 2026-10-04
*Primer hito estable y oficial del motor CLI de LexiShield con soporte multiplataforma completo.*

### Añadido
- Integración de `mimalloc` y optimizaciones severas para binarios más pequeños (`strip`).
- Sistema de multihilo mediante `rayon` para procesar ficheros masivos y directorios completos.
- Framework de pruebas continuo (Unit tests, fuzz testing y benchmarking con `Criterion`).
- Soporte para reglas dinámicas cargables vía ficheros `.toml` y lectura al estilo `ripgrep` ignorando ocultos/git.
- Pipelines de compilación definitivos (GitHub Actions y GitLab CI).

## [0.5.0] - 2026-10-03
*Hito de evolución de consola interactiva y cifrado.*

### Añadido
- **Bóvedas Encriptadas (Vaults)**: Gestión de mapeos cifrados usando `Argon2id` y `ChaCha20-Poly1305` con comandos `rekey`, `remove` y `delete`.
- **CLI Interactivo**: Interfaz paso a paso con paginación, indicador de progreso (bytes) y omitido selectivo de palabras durante el escaneo.
- **Detectores Avanzados**: Prevención contra colisión de subredes privadas (IPv4), protección robusta contra identificadores JSON, marcas de tiempo y pseudónimos de teléfonos.

### Optimizado
- Mejora de detección de rangos y superposiciones de datos a tiempo O(log N).
- Previsto el salto inteligente sobre ficheros binarios y comprimidos.

## [0.1.0] - 2026-10-02
*Nacimiento del proyecto en Rust.*

### Añadido
- Reesctritura e inicialización del proyecto (porting original) de Python a un motor Rust ultra-rápido.
- CLI básico con integración nativa al portapapeles y modo *watch*.
- Sistema de compilación cruzada aislado usando Vagrant (Ubuntu, macOS, Windows).
- Integración de iconos nativos e información (metadata PE) en ejecutables de Windows.
- División técnica de la documentación corporativa y arquitectónica (`docs/`).
