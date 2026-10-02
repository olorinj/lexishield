# Arquitectura Interna de LexiShield

Este documento detalla la estructura interna, las decisiones de diseño y el funcionamiento técnico del motor de LexiShield en Rust.

## 🏗️ Arquitectura de Componentes

El software en Rust se divide en capas modulares bien delimitadas:

* **Presentación (CLI):** Gestiona los argumentos por consola mediante `clap`, proporcionando una experiencia rápida y ergonómica.
* **Motor de ejecución (Engine):** Procesa flujos de datos estructurados. Separa la lógica de reemplazo semántico, gestionando el diccionario en un archivo local JSON o en memoria.
* **Validadores y Detectores (Detectors):** Módulos altamente especializados (`network.rs`, `identity.rs`, `o365.rs`, `windows.rs`) que actúan como núcleo de clasificación, detectando tipos de datos e invocando sus respectivas reglas de validación y generación sintética.
* **Formateadores (Format Adapters):** Soporte nativo y estructurado para adaptar inteligentemente la lectura y escritura según el tipo de archivo (Logs crudos, JSON, XML).

---

## ⚙️ Características Técnicas de Soporte

### Mapeo Semántico e Identificación de Tokens de Privacidad
El motor realiza una identificación precisa de la información sensible mediante un pipeline estricto en Rust:

1. **Escaneo por Expresiones Regulares:** Búsqueda en texto utilizando patrones regex avanzados y optimizados compilados en memoria.
2. **Validación Semántica Adicional:** Filtrado activo para eliminar falsos positivos mediante código (ej. validación matemática rigurosa de identificadores fiscales o validación RFC para direcciones IP).
3. **Clasificación y Reemplazo:** Asignación de la categoría semántica correspondiente priorizando los datos de alta sensibilidad antes de invocar la generación sintética.

#### Detalles de Patrones y Regex Incorporados:
* **IP Address (IPv4 / IPv6)**: Detecta y valida direcciones de red, evitando IPs de loopback genéricas si se desea.
* **Email Address**: Direcciones de correo electrónico standard, generando correos ofuscados manteniendo en lo posible el dominio si fuera necesario u ocultándolo por completo.
* **Windows SID Domain / Account**: Identificadores de seguridad SID de Windows. Consumen una lógica especializada en `windows.rs`.
* **Microsoft 365 / Entra ID**: Detección de cuentas UPN, tokens y accesos organizacionales en `o365.rs`.
* **GUID / UUID**: Identificadores únicos globales transformados criptográficamente.

### 🛡️ Generación Inteligente de Pseudónimos (Mapeo Coherente de Tipos)
LexiShield garantiza que el valor ofuscado generado sea de la misma naturaleza e igual de válido que el original para mantener la coherencia en el análisis de logs. Un UUID será sustituido por un UUID estructuralmente válido diferente; un identificador fiscal español mantendrá su letra de control validada, y una IP conservará el aspecto de una IPv4 o IPv6 según corresponda.

### 📖 Tratamiento de Caracteres Especiales y Formatos Estructurados (XML/JSON)

Para que LexiShield funcione correctamente sin importar cómo estén escritos los archivos, la versión de Rust implementa adaptadores de formato (`json_adapter`, `xml_adapter`, `log_adapter`). 

Cuando se analiza un archivo JSON o XML, el sistema no escanea ciegamente todo el texto rompiendo la estructura de etiquetas. En su lugar, analiza semánticamente los campos de datos y ofusca su contenido preservando estrictamente el esqueleto y la sintaxis del archivo original. Así, puedes subir un JSON ofuscado a una IA y devolverá un JSON completamente válido que luego puedes deserializar en tu código real.
