# Changelog

Todos los cambios notables de este proyecto se documentarán en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
