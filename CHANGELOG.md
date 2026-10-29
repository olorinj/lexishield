# Changelog

Todos los cambios notables de este proyecto se documentarán en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.0.0/),
y este proyecto se adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.0] - 2026-10-29
*Actualización integral de experiencia de usuario: Renombrado conceptual a Transformación, refactorización de barras secundarias Tier 2 (Smartbars), resiliencia multiplataforma de portapapeles en Linux (Wayland/X11/KDE), búsqueda reactiva con botón de vaciado rápido y alineación integral de documentación.*

### Añadido
- **🔍 Botón de vaciado rápido en filtro de búsqueda de Pares**:
  - Incorporación de botón reactivo `✕` en el campo de búsqueda de tokens para limpiar filtros instantáneamente.
- **⚡ Modo Markdown con ofuscación reactiva automática**:
  - Al conmutar al modo Markdown en Transformación teniendo texto claro cargado, la aplicación genera reactivamente la ofuscación Markdown sin requerir clics adicionales.
- **📋 Acceso directo de monitorización Tier 2 en Portapapeles**:
  - Integración del conmutador de inicio/detención de escucha (`Monitorizar` / `Detener`) directamente en la barra secundaria Smartbar Tier 2 de Portapapeles.
- **✨ Fusión Arquitectónica de Escáner y Studio**:
  - Unificación completa de la experiencia de usuario en la pestaña principal **Transformación**:
    - *Transformación Bidireccional*: Paneles simétricos de *Texto claro* y *Texto protegido* con puente direccional interactivo, soporte Drag & Drop dividido y telemetría de caracteres/líneas.
    - *Desglose y Auditoría*: Tabla categorizada de hallazgos (Red, Identidad, Sistemas, Contacto/Claves), barra de distribución cromática, filtrado por tipo y botón de adición directa de correspondencias a la bóveda.
- **📋 Cadena de Respaldo Híbrida de Portapapeles en Linux (Wayland / X11 / KDE)**:
  - Implementación en Rust (`src/clipboard.rs`) de mecanismos automáticos de fallback cuando `arboard` falla por inaccesibilidad del servidor X11:
    - *Wayland*: Ejecución de `wl-paste` y `wl-copy` mediante pipes estándar.
    - *KDE Plasma*: Interacción nativa con el demonio Klipper mediante llamadas D-Bus con `qdbus6` / `qdbus`.
    - *X11*: Respaldo clásico mediante utilidades `xclip` y `xsel`.
  - El bucle CLI (`watch_clipboard_loop`) inicializa el portapapeles de manera resiliente sin abortar la ejecución ante fallos de backend nativo.
  - Banner informativo y diagnóstico activo en la interfaz gráfica ante problemas de permisos o paquetes faltantes en Linux.
- **⏱️ Historial Temporal de Sesión en Memoria RAM**:
  - Almacenamiento local volátil (Zero-Knowledge) de las últimas 30 transformaciones en Transformación, permitiendo restaurar estados anteriores con un solo clic.
- **⌨️ Atajos de Teclado Centralizados en el Sistema de Ayuda**:
  - Pestaña dedicada *⌨️ Atajos de Teclado* en el modal de Ayuda contextual, invocable en cualquier momento mediante la tecla `?` o el botón de ayuda.
- **🛡️ Gestión Persistente de Perfiles de Sensibilidad en Opciones**:
  - Selector de perfiles (*Completo*, *DevOps & Red*, *RGPD & Identidad*) ubicado en la pestaña **Opciones**, con persistencia local automática (`localStorage`) entre sesiones.

### Modificado
- **🎯 Renombrado de Pestaña Principal a Transformación y Armonización Conceptual**:
  - Sustitución unificada del término *Studio* por **Transformación** en navegación Tier 1, textos explicativos, descripciones de configuración y documentación técnica.
  - Simplificación de la barra Tier 2 de Transformación: reubicación de la acción **Limpiar todo** a la barra superior secundaria y eliminación de redundancias.
  - Retirada de botones duplicados de restauración y auto-copiado en la zona inferior de paneles para una interfaz más despejada.
  - Actualización de marcadores de posición y ayudas para evitar sugerir escritura manual errática cuando los paneles actúan como receptores directos de pegado o archivos.
- **💾 Estandarización de Etiquetas de Respaldo en Bóvedas**:
  - Botones unificados a **Backup LexiShield** y **Restaurar LexiShield** tanto en barras secundarias como en modales de confirmación para eliminar ambigüedades con archivos `.bak` sueltos.
- **📐 Estandarización de Escala y Contenedores de Iconos (5 Niveles Unificados)**:
  - Normalización geométrica de todos los iconos de la interfaz en [ui/src/App.svelte](file:///home/olorinj/DEV/rust/lexishield/ui/src/App.svelte) para eliminar discrepancias de tamaño entre sistemas operativos:
    - *Nivel 1 (Botones de acción cuadrados)*: Normalizados a `w-8 h-8 rounded-xl text-sm flex items-center justify-center` (cabecera general, Transformación, puente direccional, cierres de modales).
    - *Nivel 2 (Banners informativos y consejos)*: Tipos e iconos alineados uniformemente a `text-base shrink-0`.
    - *Nivel 3 (Cápsulas de cabecera de modales)*: Tarjetas distintivas unificadas a `w-10 h-10 rounded-xl text-xl flex items-center justify-center`.
    - *Nivel 4 (Estados vacíos)*: Estandarizados a `text-3xl mb-2`.
    - *Nivel 5 (Botones de descartar consejos)*: Homogeneizados a `text-xs px-2 py-0.5 rounded`.
- **🎨 Ergonomía y Limpieza Visual de la Barra Superior de Transformación**:
  - Botón de auto-copiado al portapapeles rediseñado a formato de conmutador cuadrado (`w-8 h-8`) con cambio dinámico de icono (`📋` activo / `📄` inactivo).
  - Reubicación de la acción **"Auditar Texto"** dentro de la cabecera del panel **"Texto claro"**, donde reside el contexto operativo de los datos en claro.
  - Retirada de desplegables y botones redundantes de la cabecera superior para evitar desbordamientos horizontales.

### Corregido
- **🛡️ Preservación de Edición en Paneles de Transformación**:
  - Corrección de la reactividad que desmontaba el elemento `<textarea>` al pulsar la primera tecla si las etiquetas de tokens estaban activadas, garantizando ahora escritura y pegado estables sin pérdida de foco.
- **🙈 Cobertura Universal del Modo Privacidad (*Privacy Blur*)**:
  - Aplicado desenfoque condicional a los valores originales confidenciales en la tabla de la pestaña **Pares de Tokens**.

### Corregido
- **🙈 Cobertura Universal del Modo Privacidad (*Privacy Blur*)**:
  - Aplicado desenfoque condicional a los valores originales confidenciales en la tabla de la pestaña **Pares de Tokens**.
  - Desenfoque aplicado al campo de texto original en el modal **Añadir Par**.
  - Desenfoque aplicado al texto capturado y a las fichas de tokens en el historial del **Portapapeles**.
- **🔍 Observabilidad en el Sondeo de Portapapeles**:
  - Eliminado el silenciamiento de errores en `startWatchPolling()`; las incidencias de acceso al portapapeles ahora se notifican de inmediato en la barra de estado y en la vista de portapapeles.

## [2.0.1] - 2026-10-21
*Actualización de compatibilidad CI/CD y motor contextual estructurado para eventos de seguridad y SIEM (Wazuh / Windows EventLog / Sysmon), modernización estética y ergonomía.*

### Añadido
- **🛡️ Adaptador JSON Jerárquico y Perfil SIEM / Wazuh / Windows Events (`json_adapter.rs`)**:
  - Detección e inspección semántica mediante rutas jerárquicas (JSON Path) para eventos de seguridad.
  - Clasificación forzada y generación de pseudónimos para equipos y servidores (`agent.name`, `manager.name`, `workstationName`, `system.computer`).
  - Mapeo automático de identidades Windows: usuarios (`targetUserName`, `subjectUserName`), dominios NetBIOS (`targetDomainName`, `domain`) y SIDs de cuentas (`targetUserSid`, `subjectUserSid`).
  - Anonimización adaptativa de cabeceras de correo `InternetMessageId` (RFC 5322), preservando el sufijo del dominio base corporativo (`*.service-*.example.com`).
  - Lista de protección universal de SIDs estándar de Windows (`S-1-0-0`, `S-1-1-0`, `S-1-5-18`, etc.) que jamás son ofuscados.
- **🔒 Prevención Inteligente de Falsos Positivos y Falsos Negativos**:
  - Protección de claves y máscaras técnicas hexadecimales (`keywords`, `targetLogonId`, `subjectLogonId`, `eventId`, etc.) condicionada al contexto del árbol JSON o formato de flag (`0x...`).
  - Garantiza que campos `keywords` en JSONs arbitrarios de negocio o marketing continúen siendo escaneados y ofuscados sin omisiones indebidas.
- **🔄 Versionado Dinámico Basado en Ramas Git**:
  - Sincronización automática de versiones en compilaciones locales ([`compilar.sh`](file:///home/olorinj/DEV/rust/lexishield/compilar.sh) y [`compilar.ps1`](file:///home/olorinj/DEV/rust/lexishield/compilar.ps1)) asociadas al nombre de la rama activa (ej. `v2.0.1`).
- **🔄 Flujo Bidireccional y Paneles Interactivos en Studio (`Desofuscado` y `Ofuscado`)**:
  - Paneles renombrados a **Desofuscado** (texto en claro / original) y **Ofuscado** (texto protegido / seudónimos), ambos con capacidad de edición directa y telemetría en tiempo real.
  - Restricción y activación inteligente de acciones: pegar o escribir en *Desofuscado* habilita exclusivamente el modo **Ofuscar** (`Desofuscado ➔ Ofuscado`); pegar o escribir en *Ofuscado* habilita exclusivamente el modo **Desofuscar** (`Ofuscado ➔ Desofuscado`).
  - Puente central interactivo entre paneles con botón direccional dinámico (`[ 🛡️ Ofuscar ➔ ]` / `[ ⮜ Desofuscar 🔄 ]`) y conmutador manual de dirección `[ ⇄ ]`.
  - Arrastrar y soltar (*Drag & Drop*) contextual dividido: soltar un archivo sobre el panel *Desofuscado* ejecuta la ofuscación (`reverse: false`); soltar sobre el panel *Ofuscado* ejecuta la desofuscación (`reverse: true`) restaurando los datos con la bóveda.
  - Botones de archivo directos en la cabecera de cada panel (*"Ofuscar Fichero..."* y *"Desofuscar Fichero..."*).
- **🗔 Ergonomía de Escritorio y Bandeja del Sistema (System Tray)**:
  - Soporte nativo para minimizar y residir en la bandeja del sistema (junto al reloj de Windows), permitiendo ocultar y restaurar la ventana al hacer clic en el icono o cerrar/minimizar la aplicación.
  - Soporte integral de Arrastrar y Soltar (*Drag & Drop*) para analizar archivos o directorios completos arrastrándolos directamente sobre la vista del Escáner o sobre Studio.
  - Nuevo panel interactivo en el Escáner para pegar y auditar texto al vuelo con botón de puente rápido hacia Studio (*"Enviar a Studio"*).
  - Activada por defecto la opción *"Guardar en copia sufijada"* (`copyMode = true`) en Studio para evitar sobreescritura accidental de archivos de origen.
  - El botón *"Limpiar"* de Studio restablece ahora de forma atómica tanto la entrada original como el resultado procesado.
  - Botón *"Limpiar"* en el panel de texto del Escáner y botón *"Limpiar Resultados"* en la barra de acciones para reiniciar la vista sin perder el contexto.
  - Configuración personalizada del método de ingesta de archivos y carpetas en Opciones (`file_selection_mode`: `"ambos"`, `"arrastrar"`, `"botones"`), exigiendo que al menos uno de los métodos permanezca siempre habilitado.
  - **Modo Privacidad en Pantalla (*Privacy Blur*)**: botón rápido de ocultación en cabecera para compartir pantalla en videollamadas desenfocando los valores confidenciales originales, con revelado temporal al pasar el cursor.
  - **Personalización Completa de Ergonomía**: selector de tamaño de fuente (`small`, `medium`, `large`), tema visual persistente (`system`, `dark`, `light`), control de minimizado a bandeja al pulsar `[X]` (`close_to_tray`) y sufijo personalizable para copias procesadas (`output_suffix`).
  - **Auto-Bloqueo de Bóveda por Inactividad**: temporizador configurable (1, 5, 15 o 30 minutos) que descarga de forma segura de la memoria RAM la contraseña y los mapeos ante la inactividad del usuario.
  - **🎨 Modernización Visual y Estética de la Interfaz (v2.0.1 UI Modernization)**:
    - *Paletas de Color de Acento Dinámicas*: 5 temas de acento seleccionables (Cian Neón, Esmeralda Ciber, Azul Cobalto, Amatista y Ámbar Retro) que reconfiguran en caliente todas las utilidades primarias mediante variables CSS dinámicas (`[data-accent]`).
    - *Editor Studio tipo IDE*: Canal lateral de numeración de líneas sincronizado al desplazamiento vertical (`gutter`), insignia de detección sintáctica (`JSON Wazuh`, `JSON`, `XML / HTML`, `LOG / SYSLOG`, `TEXTO PLANO`), telemetría en tiempo real (líneas, caracteres, KB) y feedback de confirmación animado al copiar (`✓ ¡Copiado!`).
    - *Barra de Distribución Categórica en el Escáner*: Segmentación horizontal de hallazgos inspirada en la barra de lenguajes de GitHub, agrupada por categorías semánticas (Red 🌐, Identidad 🪪, Sistemas 🖥️ y Contacto/Claves 💳) con insignias cromáticas e iconos contextuales en la tabla.
    - *Acabado Fluent y Frosted Acrylic Cards*: Tarjetas con profundidad y relieve sutil (`frosted-card`), e iluminación dinámica con borde resplandeciente (`pulse-glow`) en el área de arrastrar y soltar (*Drag & Drop*).
    - *Barra de Estado Modular Segmentada*: Pie de ventana transformado en cápsulas modulares tipo VS Code / JetBrains que reportan el estado en vivo de la bóveda, arquitectura Zero-Knowledge, formato sintáctico activo, rendimiento del procesador, privacidad y versión.
- **📄 Soporte Nativo de Flujos Multi-Alerta NDJSON (*JSON Lines*)**:
  - Detección, procesamiento y desofuscación integral de flujos con múltiples objetos JSON consecutivos (formato estándar de logs Wazuh `/var/ossec/logs/alerts/alerts.json`), extrayendo mapeos contextuales de todo el flujo y reconstruyendo el formato línea por línea con validación sintáctica completa.
  - Flujo de trabajo ágil en el Escáner: notificaciones no bloqueantes en línea y mantenimiento del panel de texto abierto si se obtienen cero detecciones.

### Mejorado
- **📦 Actualización de Infraestructura CI/CD en GitHub Actions**:
  - Actualización de `actions/checkout` y `actions/setup-node` a **v5** en todos los flujos de integración continua (`ci.yml`, `release.yml`, `release-cli.yml`, `ci-release-builds.yml`), eliminando advertencias de obsolescencia y adaptándose a entornos Node.js 24.
- **🧪 Cobertura de Pruebas Unitarias y de Integración**:
  - Nuevas pruebas de ciclo completo para eventos de alerta Wazuh, eventos genéricos de Windows / Sysmon (`winlog.event_data`) y verificación de comportamiento en documentos de negocio, con validación de reversibilidad biyectiva al 100%.

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
