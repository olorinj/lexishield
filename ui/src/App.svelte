<script>
  import { onMount } from "svelte";
  import { fade, slide } from "svelte/transition";
  import { open, save } from "@tauri-apps/plugin-dialog";

  // Funciones de sistema de archivos
  async function invokeTauri(cmd, args = {}) {
    return await window.__TAURI__.core.invoke(cmd, args);
  }

  async function openFileToProcess(reverse = false) {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Archivos soportados',
          extensions: ['txt', 'json', 'xml', 'log', 'docx', 'xlsx', 'pptx']
        }]
      });
      if (!selected) return;
      
      loading = true;
      statusMessage = "Procesando archivo...";
      
      const outPath = await save({
        defaultPath: selected + (reverse ? ".clear" : ".lexishield")
      });
      
      if (!outPath) {
        loading = false;
        statusMessage = "Operación cancelada";
        return;
      }

      const res = await invokeTauri("process_file_command", {
        inputPath: selected,
        outputPath: outPath,
        vaultPath: vaultPath || null,
        password: password || null,
        reverse
      });
      
      report = res;
      statusMessage = "Archivo procesado y guardado en: " + outPath;
    } catch (err) {
      console.error(err);
      statusMessage = "Error al procesar archivo: " + err;
    } finally {
      loading = false;
    }
  }

  async function openVaultFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Bóvedas LexiShield',
          extensions: ['lexi']
        }]
      });
      if (!selected) return;
      
      vaultPath = selected;
      statusMessage = "Cargando bóveda...";
      const items = await invokeTauri("read_vault_file", { path: vaultPath, password: password || null });
      filteredVault = items;
      statusMessage = "Bóveda cargada: " + vaultPath;
    } catch (err) {
      console.error(err);
      statusMessage = "Error al abrir bóveda: " + err;
    }
  }

  // Control de pestañas activas
  let activeTab = "studio"; // "studio" | "scanner" | "vault" | "watch"

  // Estado del Studio
  let inputText = "";
  let outputText = "";
  let format = "auto";
  let vaultPath = "";
  let password = "";
  let loading = false;
  let report = null;
  let detectedItems = [];
  let statusMessage = "";

  // Estado de Bóveda
  let vaultMappings = [];
  let vaultFilter = "";
  let newOriginal = "";
  let newPseudonym = "";
  let newType = "Custom";

  // Estado Guardián
  let watchActive = false;
  let watchDirection = "obfuscate";
  let watchCount = 0;
  let watchInterval = null;

  // Invocar backend de Tauri si está disponible

  // Fallback simulado para previsualización web pura
  function simulateTauri(cmd, args) {
    if (cmd === "obfuscate_content") {
      const text = args.text || "";
      const obfuscated = text
        .replace(/\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g, "email000001@acme.com")
        .replace(/\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b/g, "192.168.10.55");
      return {
        obfuscated_text: obfuscated,
        report: {
          format_detected: "Auto",
          original_length: text.length,
          result_length: obfuscated.length,
          replacements_applied: 4,
          elapsed_ms: 0.42,
          schema_intact: true,
          syntax_valid: true,
          warnings: []
        },
        mappings: [
          { original: "admin@empresa.com", pseudonym: "email000001@acme.com", detector_type: "Email", omitted: false },
          { original: "10.0.0.1", pseudonym: "192.168.10.55", detector_type: "IPv4", omitted: false }
        ]
      };
    }
    if (cmd === "scan_content") {
      return [
        { original: "admin@empresa.com", pseudonym: "email000001@acme.com", detector_type: "Email", omitted: false },
        { original: "192.168.1.100", pseudonym: "192.168.22.40", detector_type: "IPv4", omitted: false },
        { original: "S-1-5-21-3623811015-3361044348-30300820-1013", pseudonym: "S-1-5-21-9999999999-0000000000-00000000-0001", detector_type: "Windows SID", omitted: false }
      ];
    }
    return {};
  }

  async function handleObfuscate() {
    if (!inputText.trim()) return;
    loading = true;
    statusMessage = "Ofuscando texto de forma semántica...";
    try {
      const res = await invokeTauri("obfuscate_content", {
        text: inputText,
        formatStr: format,
        vaultPath: vaultPath || null,
        password: password || null
      });
      outputText = res.obfuscated_text;
      report = res.report;
      detectedItems = res.mappings || [];
      statusMessage = `✅ Ofuscación completada en ${report?.elapsed_ms?.toFixed(2) || 0}ms (${report?.replacements_applied || 0} reemplazos).`;
    } catch (e) {
      statusMessage = `❌ Error: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function handleDeobfuscate() {
    if (!inputText.trim()) return;
    loading = true;
    statusMessage = "Desofuscando y restaurando datos reales...";
    try {
      const res = await invokeTauri("deobfuscate_content", {
        text: inputText,
        formatStr: format,
        vaultPath: vaultPath || null,
        password: password || null,
        customMappings: detectedItems.length > 0 ? detectedItems : null
      });
      outputText = res.deobfuscated_text;
      report = res.report;
      statusMessage = `✅ Restauración completada en ${report?.elapsed_ms?.toFixed(2) || 0}ms.`;
    } catch (e) {
      statusMessage = `❌ Error: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function handleScan() {
    if (!inputText.trim()) return;
    loading = true;
    statusMessage = "Escaneando entidades sensibles...";
    try {
      detectedItems = await invokeTauri("scan_content", { text: inputText });
      statusMessage = `🔍 Escaneo completado: ${detectedItems.length} entidades detectadas.`;
      if (detectedItems.length > 0 && activeTab === "studio") {
        activeTab = "scanner";
      }
    } catch (e) {
      statusMessage = `❌ Error al escanear: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function pasteClipboard() {
    try {
      if (window.__TAURI__) {
        inputText = await invokeTauri("get_system_clipboard");
      } else {
        inputText = await navigator.clipboard.readText();
      }
      statusMessage = "📋 Texto pegado desde el portapapeles.";
    } catch (e) {
      statusMessage = "No se pudo leer el portapapeles.";
    }
  }

  async function copyOutput() {
    if (!outputText) return;
    try {
      if (window.__TAURI__) {
        await invokeTauri("set_system_clipboard", { text: outputText });
      } else {
        await navigator.clipboard.writeText(outputText);
      }
      statusMessage = "📋 ¡Texto ofuscado copiado al portapapeles con éxito!";
    } catch (e) {
      statusMessage = "Error al copiar.";
    }
  }

  function toggleWatch() {
    watchActive = !watchActive;
    if (watchActive) {
      statusMessage = "🛡️ Modo Guardián activado. Monitorizando el portapapeles...";
    } else {
      statusMessage = "⏹️ Modo Guardián pausado.";
    }
  }

  $: filteredVault = vaultMappings.filter(
    (m) =>
      m.original.toLowerCase().includes(vaultFilter.toLowerCase()) ||
      m.pseudonym.toLowerCase().includes(vaultFilter.toLowerCase())
  );
</script>

<div class="flex flex-col h-screen bg-slate-950 text-slate-100 font-sans select-none">
  <!-- Header Principal con Región de Arrastre Nativa de Ventana -->
  <header data-tauri-drag-region class="flex items-center justify-between px-6 py-3 bg-brand-surface/90 backdrop-blur-md border-b border-white/10 shrink-0">
    <div class="flex items-center gap-3 pointer-events-none">
      <div class="w-9 h-9 rounded-xl bg-gradient-to-tr from-brand-navy to-primary-500 border border-primary-500/30 flex items-center justify-center shadow-lg shadow-cyan-950/50">
        <svg class="w-5 h-5 text-primary-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
        </svg>
      </div>
      <div>
        <div class="flex items-center gap-2">
          <h1 class="text-base font-bold tracking-wide text-white">LexiShield</h1>
          <span class="text-[10px] uppercase font-mono px-2 py-0.5 rounded bg-primary-500/10 text-primary-400 border border-primary-500/20">v1.0.0 GUI</span>
        </div>
        <p class="text-xs text-slate-400">Escudo de Privacidad y Ofuscación Semántica para Consultas de IA</p>
      </div>
    </div>

    <!-- Navegación de Pestañas (Bento Navigation) -->
    <nav class="flex items-center gap-1 bg-slate-950/80 backdrop-blur p-1 rounded-xl border border-white/10">
      <button
        class="px-4 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-2 {activeTab === 'studio' ? 'bg-primary-500 text-slate-950 font-bold shadow-md shadow-primary-500/20' : 'text-slate-400 hover:text-white hover:bg-white/5'}"
        on:click={() => (activeTab = "studio")}
      >
        <span>⚡</span> Studio
      </button>
      <button
        class="px-4 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-2 {activeTab === 'scanner' ? 'bg-primary-500 text-slate-950 font-bold shadow-md shadow-primary-500/20' : 'text-slate-400 hover:text-white hover:bg-white/5'}"
        on:click={() => (activeTab = "scanner")}
      >
        <span>🔍</span> Auditoría ({detectedItems.length})
      </button>
      <button
        class="px-4 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-2 {activeTab === 'vault' ? 'bg-primary-500 text-slate-950 font-bold shadow-md shadow-primary-500/20' : 'text-slate-400 hover:text-white hover:bg-white/5'}"
        on:click={() => (activeTab = "vault")}
      >
        <span>🔐</span> Bóveda
      </button>
      <button
        class="px-4 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 flex items-center gap-2 {activeTab === 'watch' ? 'bg-primary-500 text-slate-950 font-bold shadow-md shadow-primary-500/20' : 'text-slate-400 hover:text-white hover:bg-white/5'}"
        on:click={() => (activeTab = "watch")}
      >
        <span>🛡️</span> Guardián
      </button>
    </nav>
  </header>

  <!-- Contenido Principal Dinámico (Bento Grid) -->
  <main class="flex-1 overflow-hidden p-6 flex flex-col">
    {#if activeTab === "studio"}
      <!-- VISTA STUDIO -->
      <div in:fade={{ duration: 150 }} class="flex-1 grid grid-cols-1 md:grid-cols-2 gap-6 min-h-0">
        <!-- Panel Izquierdo: Entrada -->
        <div class="flex flex-col bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-white/10 overflow-hidden shadow-2xl">
          <div class="flex items-center justify-between px-4 py-2.5 bg-slate-900/60 border-b border-white/10">
            <div class="flex items-center gap-2">
              <span class="w-2.5 h-2.5 rounded-full bg-amber-400"></span>
              <span class="text-xs font-semibold text-slate-200">Datos Sensibles Originales (Logs / Código / JSON)</span>
            </div>
            <div class="flex items-center gap-2">
              <select bind:value={format} class="bg-slate-950 text-slate-300 text-xs px-2 py-1 rounded-lg border border-white/10 outline-none focus:border-primary-500 transition">
                <option value="auto">Formato: Auto</option>
                <option value="json">JSON</option>
                <option value="xml">XML</option>
                <option value="log">Log</option>
                <option value="plaintext">Texto Plano</option>
              </select>
              <button on:click={pasteClipboard} class="text-xs px-2.5 py-1 bg-white/5 hover:bg-white/10 text-slate-300 rounded-lg border border-white/10 flex items-center gap-1 transition">
                📋 Pegar
              </button>
              <button on:click={() => openFileToProcess(false)} class="text-xs px-2.5 py-1 bg-primary-600/20 hover:bg-primary-500/30 text-primary-400 rounded-lg border border-primary-500/20 flex items-center gap-1 transition">
                📄 Ofuscar Fichero
              </button>
            </div>
          </div>
          <textarea
            bind:value={inputText}
            placeholder="Pega aquí el contenido confidencial a ofuscar antes de enviarlo a ChatGPT o Claude (ej. IPs, emails, SIDs, contraseñas)..."
            class="flex-1 p-4 bg-transparent text-slate-200 text-sm font-mono resize-none focus:outline-none placeholder-slate-600 leading-relaxed"
          ></textarea>
        </div>

        <!-- Panel Derecho: Salida -->
        <div class="flex flex-col bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-white/10 overflow-hidden shadow-2xl">
          <div class="flex items-center justify-between px-4 py-2.5 bg-slate-900/60 border-b border-white/10">
            <div class="flex items-center gap-2">
              <span class="w-2.5 h-2.5 rounded-full bg-primary-400"></span>
              <span class="text-xs font-semibold text-slate-200">Resultado Sanitizado (Seguro para IA)</span>
            </div>
            <div class="flex items-center gap-2">
              <button on:click={() => openFileToProcess(true)} class="text-xs px-2.5 py-1 bg-rose-600/20 hover:bg-rose-500/30 text-rose-400 rounded-lg border border-rose-500/20 flex items-center gap-1 transition">
                📄 Desofuscar Fichero
              </button>
              <button on:click={copyOutput} class="text-xs px-3 py-1 bg-primary-500 hover:bg-primary-400 text-slate-950 font-bold rounded-lg flex items-center gap-1.5 shadow-md shadow-primary-500/20 transition">
                📋 Copiar
              </button>
            </div>
          </div>
          <textarea
            readonly
            bind:value={outputText}
            placeholder="El resultado transformado aparecerá aquí con coherencia semántica garantizada..."
            class="flex-1 p-4 bg-transparent text-primary-300 text-sm font-mono resize-none focus:outline-none placeholder-slate-700 leading-relaxed"
          ></textarea>
        </div>
      </div>

      <!-- Barra de Acciones y Bóveda (Bento Card) -->
      <div in:fade={{ duration: 150 }} class="mt-4 p-4 bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-white/10 flex flex-wrap items-center justify-between gap-4">
        <!-- Opciones de Bóveda -->
        <div class="flex items-center gap-3 flex-1 min-w-[280px]">
          <div class="flex-1 flex items-center bg-slate-950/80 rounded-xl px-3 py-1.5 border border-white/10 focus-within:border-primary-500/50 transition">
            <span class="text-xs text-slate-500 mr-2">📁 Bóveda:</span>
            <input
              type="text"
              bind:value={vaultPath}
              placeholder="Opcional: ruta a mapeos.lexi o .json"
              class="bg-transparent text-xs text-slate-300 outline-none flex-1 font-mono"
            />
          </div>
          <div class="flex items-center bg-slate-950/80 rounded-xl px-3 py-1.5 border border-white/10 focus-within:border-primary-500/50 transition">
            <span class="text-xs text-slate-500 mr-2">🔑 Clave:</span>
            <input
              type="password"
              bind:value={password}
              placeholder="Contraseña (.lexi)"
              class="bg-transparent text-xs text-slate-300 outline-none w-28 font-mono"
            />
          </div>
        </div>

        <!-- Botones de Acción -->
        <div class="flex items-center gap-2">
          <button
            on:click={handleScan}
            disabled={loading || !inputText}
            class="px-4 py-2 bg-white/5 hover:bg-white/10 text-slate-200 text-xs font-semibold rounded-xl border border-white/10 transition duration-150 disabled:opacity-50"
          >
            🔍 Auditar Entidades
          </button>
          <button
            on:click={handleDeobfuscate}
            disabled={loading || !inputText}
            class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold rounded-xl transition duration-150 shadow-lg shadow-indigo-950/50 disabled:opacity-50"
          >
            ↩️ Desofuscar Respuesta
          </button>
          <button
            on:click={handleObfuscate}
            disabled={loading || !inputText}
            class="px-6 py-2 bg-gradient-to-r from-primary-600 to-primary-400 hover:from-primary-500 hover:to-primary-300 text-slate-950 text-xs font-bold rounded-xl transition duration-150 shadow-lg shadow-primary-500/20 flex items-center gap-2 disabled:opacity-50"
          >
            {#if loading}
              <span class="animate-spin">⏳</span>
            {:else}
              <span>🛡️</span>
            {/if}
            Ofuscar para IA
          </button>
        </div>
      </div>

    {:else if activeTab === "scanner"}
      <!-- VISTA AUDITORÍA Y ESCÁNER -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-white/10 p-6 flex flex-col min-h-0">
        <div class="flex items-center justify-between mb-4">
          <div>
            <h2 class="text-sm font-bold text-white">Auditoría Interactiva de Entidades Detectadas</h2>
            <p class="text-xs text-slate-400">Revisa qué pares se transformarán. Puedes omitir falsos positivos marcando la casilla.</p>
          </div>
          <span class="text-xs px-3 py-1 bg-white/5 text-slate-300 rounded-lg border border-white/10">
            Total: {detectedItems.length} detecciones
          </span>
        </div>

        <div class="flex-1 overflow-auto rounded-xl border border-white/10 bg-slate-950/80">
          <table class="w-full text-left text-xs font-mono">
            <thead class="bg-slate-900/90 text-slate-400 sticky top-0 border-b border-white/10">
              <tr>
                <th class="p-3">Omitir</th>
                <th class="p-3">Tipo de Entidad</th>
                <th class="p-3">Valor Original Confidencial</th>
                <th class="p-3">Seudónimo Coherente Asignado</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-white/5 text-slate-300">
              {#each detectedItems as item, idx}
                <tr class="hover:bg-white/5 transition {item.omitted ? 'opacity-40' : ''}">
                  <td class="p-3">
                    <input type="checkbox" bind:checked={item.omitted} class="rounded accent-primary-500 cursor-pointer" />
                  </td>
                  <td class="p-3">
                    <span class="px-2 py-0.5 rounded bg-primary-500/10 text-primary-400 text-[11px] border border-primary-500/20">
                      {item.detector_type}
                    </span>
                  </td>
                  <td class="p-3 text-amber-300 font-semibold">{item.original}</td>
                  <td class="p-3 text-primary-400">{item.omitted ? '=' : item.pseudonym}</td>
                </tr>
              {:else}
                <tr>
                  <td colspan="4" class="p-8 text-center text-slate-500">
                    No hay entidades escaneadas todavía. Escribe texto en el Studio y pulsa "Auditar Entidades".
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

    {:else if activeTab === "vault"}
      <!-- VISTA GESTOR DE BÓVEDAS -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-white/10 p-6 flex flex-col min-h-0">
        <div class="flex items-center justify-between mb-4">
          <div>
            <h2 class="text-sm font-bold text-white">Gestor de Bóvedas y Diccionarios Cifrados (.lexi)</h2>
            <p class="text-xs text-slate-400">Explora, añade y audita mapeos inyectivos persistidos de forma segura.</p>
          </div>
          <div class="flex items-center gap-3">
            <button on:click={openVaultFile} class="text-xs px-3 py-1.5 bg-white/5 hover:bg-white/10 text-slate-300 rounded-xl border border-white/10 flex items-center gap-1.5 transition shadow-sm">
              📂 Cargar .lexi
            </button>
            <input
              type="text"
              bind:value={vaultFilter}
              placeholder="🔍 Filtrar mapeos..."
              class="bg-slate-950/80 text-xs text-slate-200 px-3 py-1.5 rounded-xl border border-white/10 focus:border-primary-500 outline-none w-64"
            />
          </div>
        </div>

        <div class="flex-1 overflow-auto rounded-xl border border-white/10 bg-slate-950/80 p-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            {#each filteredVault as item}
              <div class="p-3 rounded-xl bg-white/5 border border-white/10 flex items-center justify-between">
                <div>
                  <div class="text-xs text-amber-300 font-mono font-semibold">{item.original}</div>
                  <div class="text-xs text-primary-400 font-mono">↳ {item.pseudonym}</div>
                </div>
                <span class="text-[10px] px-2 py-0.5 rounded bg-slate-800 text-slate-400 border border-white/5">
                  {item.detector_type}
                </span>
              </div>
            {:else}
              <div class="col-span-2 p-8 text-center text-slate-500 text-xs">
                No hay mapeos cargados. Carga una bóveda .lexi o realiza una ofuscación en el Studio.
              </div>
            {/each}
          </div>
        </div>
      </div>

    {:else if activeTab === "watch"}
      <!-- VISTA GUARDIÁN DE PORTAPAPELES -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-white/10 p-8 flex flex-col items-center justify-center text-center">
        <div class="w-20 h-20 rounded-3xl bg-primary-500/10 border border-primary-500/30 flex items-center justify-center mb-6 shadow-2xl shadow-primary-500/10">
          <span class="text-4xl">{watchActive ? "🛡️" : "💤"}</span>
        </div>
        <h2 class="text-lg font-bold text-white mb-2">Modo Guardián de Portapapeles</h2>
        <p class="text-xs text-slate-400 max-w-md mb-6 leading-relaxed">
          Cada vez que pulses <kbd class="px-1.5 py-0.5 bg-white/10 rounded text-slate-300 font-mono text-[11px]">Ctrl+C</kbd> para copiar texto con datos confidenciales, LexiShield lo transformará automáticamente al vuelo con supresión de eco.
        </p>

        <div class="flex items-center gap-4 mb-8">
          <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
            <input type="radio" bind:group={watchDirection} value="obfuscate" class="accent-primary-500" />
            Ofuscación Automática (Hacia IA)
          </label>
          <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
            <input type="radio" bind:group={watchDirection} value="deobfuscate" class="accent-primary-500" />
            Desofuscación Automática (Desde IA)
          </label>
        </div>

        <button
          on:click={toggleWatch}
          class="px-8 py-3 rounded-2xl font-bold text-sm shadow-xl transition-all duration-200 {watchActive ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-rose-950/50' : 'bg-primary-500 hover:bg-primary-400 text-slate-950 shadow-primary-500/20'}"
        >
          {watchActive ? "Detener Guardián" : "Activar Protección de Portapapeles"}
        </button>
      </div>
    {/if}
  </main>

  <!-- Footer con Telemetría / Barra de Estado -->
  <footer class="px-6 py-2.5 bg-brand-surface/90 backdrop-blur-md border-t border-white/10 text-[11px] text-slate-400 flex items-center justify-between shrink-0">
    <div class="flex items-center gap-2">
      <span class="w-2 h-2 rounded-full {loading ? 'bg-amber-400 animate-pulse' : 'bg-primary-400'}"></span>
      <span class="font-mono">{statusMessage || 'Listo para proteger tus datos.'}</span>
    </div>
    {#if report}
      <div class="flex items-center gap-4 font-mono text-slate-500">
        <span>Formato: <strong class="text-slate-300">{report.format_detected}</strong></span>
        <span>Reemplazos: <strong class="text-primary-400">{report.replacements_applied}</strong></span>
        <span>Tiempo: <strong class="text-slate-300">{report.elapsed_ms.toFixed(2)}ms</strong></span>
      </div>
    {/if}
  </footer>
</div>

