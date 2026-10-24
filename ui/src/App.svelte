<script>
  import { onMount, onDestroy } from "svelte";
  import { fade, slide } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import packageInfo from "../package.json";

  const appVersion = packageInfo.version || "2.0.0";

  // Control de Tema Claro / Oscuro
  let darkMode = true;

  // Control de Pestañas Activas
  let activeTab = "studio"; // "studio" | "pairs" | "vaults" | "watch" | "config"

  // Configuración de la Aplicación y Ergonomía
  let appConfig = {
    gui: {
      default_tab: "studio",
      show_cli_dock: true,
      tabs: {
        show_studio: true,
        show_pairs: true,
        show_vaults: true,
        show_watch: true,
      },
      clipboard: {
        notification_type: "toast",
        auto_clear_enabled: false,
        auto_clear_seconds: 5,
      },
      file_selection_mode: "ambos",
      close_to_tray: true,
      close_action: "tray",
      output_suffix: "_obfuscated",
      font_size: "medium",
      privacy_blur: false,
      auto_lock_minutes: 0,
      theme: "system",
      accent_color: "cyan",
    },
  };

  let inputGutterEl;
  let outputGutterEl;
  let copiedOutputFlash = false;
  let deobfuscateSuccessFlash = false;

  $: fileSelectionMode = appConfig?.gui?.file_selection_mode || "ambos";
  $: allowDragDrop = fileSelectionMode === "ambos" || fileSelectionMode === "arrastrar";
  $: showFileButtons = fileSelectionMode === "ambos" || fileSelectionMode === "botones";
  $: copySuffix = appConfig?.gui?.output_suffix || "_obfuscated";
  $: privacyBlur = appConfig?.gui?.privacy_blur || false;
  $: editorFontSizeClass =
    appConfig?.gui?.font_size === "small"
      ? "text-[11px]"
      : appConfig?.gui?.font_size === "large"
      ? "text-sm"
      : "text-xs";

  $: inputLinesCount = inputText ? inputText.split("\n").length : 1;
  $: outputLinesCount = outputText ? outputText.split("\n").length : 1;
  $: inputCharCount = inputText.length;
  $: outputCharCount = outputText.length;
  $: inputSizeKB = (new TextEncoder().encode(inputText).length / 1024).toFixed(1) + " KB";
  $: outputSizeKB = (new TextEncoder().encode(outputText).length / 1024).toFixed(1) + " KB";
  $: inputGutterText = Array.from({ length: Math.min(inputLinesCount, 5000) }, (_, i) => i + 1).join("\n");
  $: outputGutterText = Array.from({ length: Math.min(outputLinesCount, 5000) }, (_, i) => i + 1).join("\n");

  $: detectedInputFormat = (() => {
    if (selectedFormat && selectedFormat !== "auto") return selectedFormat.toUpperCase();
    const trimmed = inputText.trim();
    if (!trimmed) return "AUTO";
    if (trimmed.startsWith("{") && (trimmed.includes('"wazuh"') || trimmed.includes('"rule"') || trimmed.includes('"agent"'))) return "JSON WAZUH";
    if (trimmed.startsWith("{") || trimmed.startsWith("[")) return "JSON";
    if (trimmed.startsWith("<") && trimmed.endsWith(">")) return "XML / HTML";
    if (/^(?:\w{3}\s+\d+|\d{4}-\d{2}-\d{2})/.test(trimmed) || trimmed.includes("INFO") || trimmed.includes("ERROR") || trimmed.includes("WARN")) return "LOG / SYSLOG";
    return "TEXTO PLANO";
  })();

  // Acción Svelte para autofoco confiable con reintentos para WebViews
  function autofocus(node) {
    const focusAndSelect = () => {
      try {
        if (document.activeElement !== node) {
          node.focus();
        }
        if (typeof node.select === "function") {
          node.select();
        }
      } catch (err) {
        // ignore
      }
    };

    focusAndSelect();
    const t1 = setTimeout(focusAndSelect, 50);
    const t2 = setTimeout(focusAndSelect, 150);
    const t3 = setTimeout(focusAndSelect, 300);

    return {
      destroy() {
        clearTimeout(t1);
        clearTimeout(t2);
        clearTimeout(t3);
      },
    };
  }

  function getDetectorCategory(detectorType) {
    const dt = (detectorType || "").toLowerCase();
    if (dt.includes("ip") || dt.includes("mac") || dt.includes("domain") || dt.includes("url") || dt.includes("host") || dt.includes("red")) {
      return { name: "Red", icon: "🌐", color: "bg-blue-500", text: "text-blue-500", border: "border-blue-500/30", bgBadge: "bg-blue-500/10" };
    }
    if (dt.includes("dni") || dt.includes("nie") || dt.includes("user") || dt.includes("usuario") || dt.includes("nombre") || dt.includes("ident")) {
      return { name: "Identidad", icon: "🪪", color: "bg-purple-500", text: "text-purple-500", border: "border-purple-500/30", bgBadge: "bg-purple-500/10" };
    }
    if (dt.includes("path") || dt.includes("ruta") || dt.includes("file") || dt.includes("sistema") || dt.includes("sys") || dt.includes("process")) {
      return { name: "Sistemas", icon: "🖥️", color: "bg-emerald-500", text: "text-emerald-500", border: "border-emerald-500/30", bgBadge: "bg-emerald-500/10" };
    }
    return { name: "Contacto / Claves", icon: "💳", color: "bg-amber-500", text: "text-amber-500", border: "border-amber-500/30", bgBadge: "bg-amber-500/10" };
  }

  $: visibleDetectedItems = filterItemsByProfile(detectedItems, activeSensitivityProfile);

  $: categoryStats = (() => {
    if (!visibleDetectedItems || visibleDetectedItems.length === 0) return null;
    const counts = { Red: 0, Identidad: 0, Sistemas: 0, "Contacto / Claves": 0 };
    for (const item of visibleDetectedItems) {
      const cat = getDetectorCategory(item.detector_type);
      counts[cat.name] = (counts[cat.name] || 0) + 1;
    }
    const total = visibleDetectedItems.length;
    return {
      total,
      red: { count: counts.Red, pct: ((counts.Red / total) * 100).toFixed(1) },
      identidad: { count: counts.Identidad, pct: ((counts.Identidad / total) * 100).toFixed(1) },
      sistemas: { count: counts.Sistemas, pct: ((counts.Sistemas / total) * 100).toFixed(1) },
      claves: { count: counts["Contacto / Claves"], pct: ((counts["Contacto / Claves"] / total) * 100).toFixed(1) },
    };
  })();

  async function copyOutputWithFeedback() {
    if (!outputText) return;
    try {
      await navigator.clipboard.writeText(outputText);
      copiedOutputFlash = true;
      statusMessage = "📋 Copiado al portapapeles.";
      setTimeout(() => {
        copiedOutputFlash = false;
      }, 2000);
    } catch (e) {
      statusMessage = "❌ Error al copiar: " + e;
    }
  }

  let copiedInputFlash = false;
  async function copyInputWithFeedback() {
    if (!inputText) return;
    try {
      await navigator.clipboard.writeText(inputText);
      copiedInputFlash = true;
      statusMessage = "📋 Texto desofuscado copiado al portapapeles.";
      setTimeout(() => {
        copiedInputFlash = false;
      }, 2000);
    } catch (e) {
      statusMessage = "❌ Error al copiar: " + e;
    }
  }

  // Estado del Studio
  let studioSubView = "transform"; // "transform" (Editor Bidireccional) | "audit" (Desglose y Auditoría)
  let inputText = "";
  let outputText = "";
  let selectedFormat = "auto";
  let vaultPath = "";
  let password = "";
  let loading = false;
  let statusMessage = "Listo. Pega tu texto o selecciona un archivo para procesarlo.";
  let lastReport = null;
  let vaultMappings = [];
  let studioDirection = "obfuscate"; // "obfuscate" (Texto claro -> Texto protegido) | "deobfuscate" (Texto protegido -> Texto claro)
  let studioDragTarget = "deobfuscated"; // "deobfuscated" | "obfuscated"

  function toggleStudioDirection() {
    studioDirection = studioDirection === "obfuscate" ? "deobfuscate" : "obfuscate";
    statusMessage = studioDirection === "obfuscate"
      ? "Dirección activa: Proteger (Texto claro ➔ Texto protegido)"
      : "Dirección activa: Restaurar (Texto protegido ➔ Texto claro)";
  }

  // Opciones de Ofuscación
  let copyMode = true; // Por defecto guardar en copia sufijada
  let cleanCache = false;
  let hasPandoc = false;
  let exportMarkdown = false;

  // Estado del Escáner y Drag & Drop
  let detectedItems = [];
  let auditSummary = null;
  let recursiveScan = false;
  let noGitIgnoreScan = true;
  let isScanningDirectory = false;
  let showTextScanPanel = false;
  let textToAudit = "";
  let isDragging = false;
  let unlistenDragDrop = null;
  let scanCurrentFile = "";
  let scanFilesCount = 0;
  let scanFilesIgnored = 0;
  let scanEntitiesCount = 0;
  let scanPollTimer = null;

  // Estado de la Pestaña Pares de Ofuscación
  let pairsSelectedVault = "";
  let pairSearch = "";
  let pairTypeFilter = "all";
  let showAddPairModal = false;
  let newPairOriginal = "";
  let newPairPseudo = "";
  let newPairType = "GenericToken";
  let selectedPairKeys = [];
  let editingPairOriginal = null;
  let editingPairPseudo = "";

  // Estado de la Pestaña Bóvedas Lexi y Backups
  let allVaultsList = [];
  let showCreateVaultModal = false;
  let newVaultName = "";
  let newVaultPassword = "";
  let showRenameVaultModal = false;
  let renameOldVaultName = "";
  let renameNewVaultName = "";
  let showBackupModal = false;
  let backupPassword = "";
  let backupStatus = "";

  // Estado del Desbloqueo e Inicialización de Bóveda
  let showUnlockModal = false;
  let unlockPassword = "";
  let unlockError = "";
  let unlockTargetVault = "";
  let isVaultEncrypted = false;
  let isVaultUnlocked = false;
  let showInitVaultModal = false;
  let initVaultPassword = "";
  let initVaultTarget = "";

  // Estado del Portapapeles (Watch)
  let watchDirection = "obfuscate";
  let isWatching = false;
  let watchPollInterval = null;
  let lastSeenClipboard = "";
  let clipboardFlash = false;
  let autoClearTimer = null;
  let clipboardEvents = [];
  let clipboardError = null;

  // Estado de Ayuda Contextual y Onboarding
  let activeHelpTab = null; // "studio" | "pairs" | "vaults" | "watch" | "config" | "logs" | "password_why"
  let dismissedTips = JSON.parse(localStorage.getItem("dismissed_tips") || "{}");

  // Estado de Notificación / Modal de Acción Completada
  let actionResultModal = null; // { title: string, message: string, details?: Array<{label: string, value: string}>, type?: "success" | "info" }
  let studioDropModal = null; // { path: string, name: string, isDir: boolean, isReverse: boolean, copyMode: boolean, suffix: string, saveSuffixDefault: boolean, cleanCache: boolean, exportMarkdown: boolean }

  // Mejoras Ergonómicas y Funcionales de Studio (v2.1)
  let autoCopyToClipboard = true;
  let showKeyboardShortcutsModal = false;
  let sessionHistory = [];
  let showSessionHistoryModal = false;
  let activeSensitivityProfile = "all"; // "all" | "devops" | "gdpr"
  let showTokenBadges = false;
  let studioOutputViewMode = "editor"; // "editor" | "markdown"

  // Estado de Depuración y Logs
  let debugMode = false;
  let showLogsModal = false;
  let debugLogs = [];
  let logFilterLevel = "all";
  let logSearch = "";
  let logPollingInterval = null;

  // Invocador seguro de comandos Tauri
  async function invokeTauri(cmd, args = {}) {
    return await invoke(cmd, args);
  }

  onMount(async () => {
    // 1. Cargar tema
    const savedTheme = localStorage.getItem("theme");
    if (savedTheme) {
      darkMode = savedTheme === "dark";
    } else {
      darkMode = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    updateThemeClass();
    applyAccentFromConfig();

    // 2. Cargar configuración
    await loadSettings();

    // 3. Cargar lista de bóvedas y bóveda inicial
    await refreshVaultsList();
    await loadInitialVault();

    // 4. Registrar escuchas de progreso Tauri y verificar herramientas
    try {
      hasPandoc = await invokeTauri("check_pandoc_available");
    } catch (e) {
      hasPandoc = false;
    }

    try {
      debugMode = await invokeTauri("get_debug_mode");
      await listen("scan-progress", (event) => {
        const payload = event.payload;
        if (payload) {
          scanCurrentFile = payload.current_file;
          scanFilesCount = payload.files_scanned;
          scanEntitiesCount = payload.entities_found;
          if (payload.new_items && payload.new_items.length > 0) {
            detectedItems = [...detectedItems, ...payload.new_items];
          }
        }
      });
    } catch (err) {
      console.warn("No se pudieron registrar eventos de Tauri:", err);
    }

    // 5. Escucha de eventos Drag & Drop de Tauri para arrastrar ficheros o carpetas
    try {
      const webview = getCurrentWebview();
      unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
        if (!allowDragDrop) {
          isDragging = false;
          return;
        }
        if (event.payload.type === "enter" || event.payload.type === "over") {
          isDragging = true;
          if (event.payload.position && typeof window !== "undefined") {
            const clientX = event.payload.position.x / (window.devicePixelRatio || 1);
            if (activeTab === "studio") {
              studioDragTarget = clientX < (window.innerWidth / 2) ? "deobfuscated" : "obfuscated";
            }
          }
        } else if (event.payload.type === "leave") {
          isDragging = false;
        } else if (event.payload.type === "drop") {
          isDragging = false;
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            let targetPanel = studioDragTarget;
            if (event.payload.position && typeof window !== "undefined") {
              const clientX = event.payload.position.x / (window.devicePixelRatio || 1);
              if (activeTab === "studio") {
                targetPanel = clientX < (window.innerWidth / 2) ? "deobfuscated" : "obfuscated";
              }
            }
            await handleDroppedPath(paths[0], targetPanel);
          }
        }
      });
    } catch (err) {
      console.warn("Listener de Drag & Drop de Tauri no disponible:", err);
    }

    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("keydown", resetInactivityTimer);
    window.addEventListener("mousemove", resetInactivityTimer);
    window.addEventListener("click", resetInactivityTimer);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
    window.removeEventListener("keydown", resetInactivityTimer);
    window.removeEventListener("mousemove", resetInactivityTimer);
    window.removeEventListener("click", resetInactivityTimer);
    if (inactivityTimer) clearTimeout(inactivityTimer);
    if (unlistenDragDrop) unlistenDragDrop();
    if (scanPollTimer) clearInterval(scanPollTimer);
    if (watchPollInterval) clearInterval(watchPollInterval);
    if (logPollingInterval) clearInterval(logPollingInterval);
    if (autoClearTimer) clearTimeout(autoClearTimer);
  });

  let inactivityTimer = null;

  function resetInactivityTimer() {
    if (inactivityTimer) clearTimeout(inactivityTimer);
    const mins = Number(appConfig?.gui?.auto_lock_minutes) || 0;
    if (mins > 0 && isVaultUnlocked) {
      inactivityTimer = setTimeout(lockSessionByInactivity, mins * 60 * 1000);
    }
  }

  function lockSessionByInactivity() {
    password = "";
    isVaultUnlocked = false;
    inputText = "";
    outputText = "";
    textToAudit = "";
    detectedItems = [];
    if (isVaultEncrypted) {
      unlockTargetVault = vaultPath;
      showUnlockModal = true;
    }
    statusMessage = "🔒 Bóveda y sesión bloqueadas automáticamente por inactividad.";
  }

  function handleKeydown(e) {
    if (e.key === "Escape") {
      if (activeHelpTab) {
        closeHelp();
      } else if (actionResultModal) {
        actionResultModal = null;
      } else if (studioDropModal) {
        studioDropModal = null;
      } else if (showKeyboardShortcutsModal) {
        showKeyboardShortcutsModal = false;
      } else if (showSessionHistoryModal) {
        showSessionHistoryModal = false;
      }
      return;
    }

    // Atajo: Ctrl+Enter o Cmd+Enter para Proteger o Restaurar en Studio
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      if (activeTab === "studio" && studioSubView === "transform" && !loading) {
        e.preventDefault();
        if (studioDirection === "obfuscate") {
          if (inputText.trim()) handleObfuscate();
        } else {
          if (outputText.trim()) handleDeobfuscate();
        }
      }
      return;
    }

    // Atajo: Ctrl+Shift+C para copiar resultado al portapapeles
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "C" || e.key === "c")) {
      if (activeTab === "studio") {
        e.preventDefault();
        if (studioDirection === "obfuscate") {
          copyOutputWithFeedback();
        } else {
          copyInputWithFeedback();
        }
      }
      return;
    }

    // Atajo: Ctrl+Shift+X para alternar sentido de transformación (⇄)
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "X" || e.key === "x")) {
      if (activeTab === "studio") {
        e.preventDefault();
        toggleStudioDirection();
      }
      return;
    }

    // Atajo: Ctrl+Shift+A para auditar texto actual en Studio
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "A" || e.key === "a")) {
      if (activeTab === "studio" && !loading) {
        e.preventDefault();
        scanCurrentStudioText();
      }
      return;
    }

    // Atajo: Ctrl+Shift+L para limpiar ambos paneles con guardado previo en historial
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "L" || e.key === "l")) {
      if (activeTab === "studio") {
        e.preventDefault();
        if (inputText || outputText) {
          pushToSessionHistory("clear");
          inputText = "";
          outputText = "";
          statusMessage = "Ambos paneles han sido limpiados (guardado en historial de sesión).";
        }
      }
      return;
    }

    // Atajo: ? para abrir la chuleta de atajos si no se está escribiendo en un input/textarea
    if (e.key === "?" && !["INPUT", "TEXTAREA", "SELECT"].includes(document.activeElement?.tagName)) {
      e.preventDefault();
      showKeyboardShortcutsModal = true;
    }
  }

  // Funciones Auxiliares para Mejoras de Studio
  function formatRelativeTime(dateObj) {
    if (!dateObj) return "";
    const seconds = Math.floor((new Date() - dateObj) / 1000);
    if (seconds < 60) return "hace un momento";
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `hace ${minutes} min`;
    const hours = Math.floor(minutes / 60);
    return `hace ${hours} h`;
  }

  function pushToSessionHistory(actionType, reportData = null) {
    if (!inputText.trim() && !outputText.trim()) return;
    const now = new Date();
    const timeStr = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    sessionHistory = [
      {
        id: Date.now(),
        timestamp: timeStr,
        dateObj: now,
        direction: studioDirection,
        actionType: actionType, // "obfuscate" | "deobfuscate" | "clear"
        inputText: inputText,
        outputText: outputText,
        tokensCount: reportData?.replacements_applied ?? (lastReport?.replacements_applied ?? 0),
        vaultUsed: vaultPath ? vaultPath.split('/').pop() : "Predeterminada"
      },
      ...sessionHistory
    ].slice(0, 10);
  }

  function restoreHistoryItem(item) {
    inputText = item.inputText;
    outputText = item.outputText;
    studioDirection = item.direction;
    studioSubView = "transform";
    showSessionHistoryModal = false;
    statusMessage = `⏱️ Sesión recuperada de las ${item.timestamp}`;
  }

  function filterItemsByProfile(items, profile) {
    if (!items || items.length === 0) return [];
    if (profile === "all") return items;
    if (profile === "devops") {
      return items.filter(it => {
        const cat = getDetectorCategory(it.detector_type);
        return cat.name === "Red" || cat.name === "Sistemas";
      });
    }
    if (profile === "gdpr") {
      return items.filter(it => {
        const cat = getDetectorCategory(it.detector_type);
        return cat.name === "Identidad" || cat.name === "Contacto / Claves";
      });
    }
    return items;
  }

  function escapeHtml(str) {
    if (!str) return "";
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  function renderHighlightedTokens(text, isProtected = false) {
    if (!text) return "";

    if (isProtected) {
      // Reemplazo en una sola pasada sobre el texto escapado
      let escaped = escapeHtml(text);
      return escaped.replace(/&lt;([A-Za-z0-9_]+)&gt;/g, (match, tokenName) => {
        const cat = getDetectorCategory(tokenName);
        const mapping = vaultMappings.find(m => m.pseudonym === `<${tokenName}>` || m.pseudonym === tokenName);
        const origTooltip = mapping ? `Original: ${mapping.original}` : `Token seudonimizado (${cat.name})`;
        return `<mark class="inline-flex items-center gap-0.5 px-1 py-0.5 rounded font-mono font-bold text-[11px] ${cat.bgBadge} ${cat.text} border ${cat.border} shadow-xs align-baseline cursor-help" title="${escapeHtml(origTooltip)}">&lt;${tokenName}&gt;</mark>`;
      });
    }

    // Para texto claro: búsqueda exacta en una sola pasada lineal sobre el string bruto original
    const knownItems = (detectedItems && detectedItems.length > 0) ? detectedItems : vaultMappings;
    if (!knownItems || knownItems.length === 0) {
      return escapeHtml(text);
    }

    // Filtrar términos no vacíos y deduplicar
    const terms = [];
    const seen = new Set();
    for (const item of knownItems) {
      if (!item.original || item.original.trim().length < 2) continue;
      if (!seen.has(item.original)) {
        seen.add(item.original);
        terms.push({
          original: item.original,
          detector_type: item.detector_type
        });
      }
    }

    if (terms.length === 0) {
      return escapeHtml(text);
    }

    // Ordenar de mayor a menor longitud para priorizar coincidencias más específicas
    terms.sort((a, b) => b.original.length - a.original.length);

    const termMap = new Map();
    const escapedTerms = [];
    for (const t of terms) {
      termMap.set(t.original, t.detector_type);
      escapedTerms.push(t.original.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
    }

    const combinedRegex = new RegExp(`(${escapedTerms.join('|')})`, 'g');
    let result = "";
    let lastIdx = 0;
    let match;

    while ((match = combinedRegex.exec(text)) !== null) {
      if (match.index > lastIdx) {
        result += escapeHtml(text.slice(lastIdx, match.index));
      }
      const matchedStr = match[0];
      const detType = termMap.get(matchedStr) || "Token";
      const cat = getDetectorCategory(detType);
      const title = `Detectado como ${cat.name} (${detType})`;
      result += `<mark class="inline-flex items-center gap-0.5 px-1 py-0.5 rounded font-mono font-bold text-[11px] ${cat.bgBadge} ${cat.text} border ${cat.border} shadow-xs align-baseline cursor-help" title="${escapeHtml(title)}">${escapeHtml(matchedStr)}</mark>`;
      lastIdx = combinedRegex.lastIndex;
    }

    if (lastIdx < text.length) {
      result += escapeHtml(text.slice(lastIdx));
    }

    return result;
  }

  function renderMarkdownPreview(rawText) {
    if (!rawText) return "<p class='text-slate-400 italic text-xs'>Sin contenido para previsualizar.</p>";
    let escaped = rawText
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");

    escaped = escaped.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, (match, lang, code) => {
      return `<pre class="my-2.5 p-3 bg-slate-900 dark:bg-slate-950 text-slate-100 rounded-xl font-mono text-xs overflow-x-auto border border-slate-700 dark:border-white/10"><code>${code.trim()}</code></pre>`;
    });

    escaped = escaped.replace(/`([^`]+)`/g, '<code class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-white/10 text-primary-600 dark:text-primary-300 font-mono text-xs">$1</code>');
    escaped = escaped.replace(/^### (.*$)/gim, '<h3 class="text-sm font-bold text-slate-900 dark:text-white mt-3 mb-1">$1</h3>');
    escaped = escaped.replace(/^## (.*$)/gim, '<h2 class="text-base font-bold text-slate-900 dark:text-white mt-4 mb-1.5 pb-1 border-b border-slate-200 dark:border-white/10">$1</h2>');
    escaped = escaped.replace(/^# (.*$)/gim, '<h1 class="text-lg font-extrabold text-slate-900 dark:text-white mt-4 mb-2 pb-1.5 border-b border-slate-300 dark:border-white/15">$1</h1>');
    escaped = escaped.replace(/^\> (.*$)/gim, '<blockquote class="my-2 pl-3 border-l-4 border-primary-500 text-slate-600 dark:text-slate-300 italic text-xs">$1</blockquote>');
    escaped = escaped.replace(/\*\*([^*]+)\*\*/g, '<strong class="font-bold text-slate-900 dark:text-white">$1</strong>');
    escaped = escaped.replace(/\*([^*]+)\*/g, '<em class="italic">$1</em>');
    escaped = escaped.replace(/^[\*\-] (.*$)/gim, '<li class="ml-4 list-disc text-xs text-slate-700 dark:text-slate-300 my-0.5">$1</li>');
    escaped = escaped.replace(/\n\n/g, '<br/><br/>').replace(/\n/g, '<br/>');

    escaped = escaped.replace(/&lt;([A-Za-z0-9_]+)&gt;/g, (match, tokenName) => {
      const cat = getDetectorCategory(tokenName);
      return `<mark class="inline-flex items-center gap-1 px-1.5 py-0.2 rounded font-mono font-bold text-[11px] ${cat.bgBadge} ${cat.text} border ${cat.border} shadow-xs" title="Token protegido &lt;${tokenName}&gt;">&lt;${tokenName}&gt;</mark>`;
    });

    return escaped;
  }

  function applyThemeFromConfig() {
    const t = appConfig?.gui?.theme || "system";
    if (t === "dark") {
      darkMode = true;
    } else if (t === "light") {
      darkMode = false;
    } else {
      darkMode = window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    updateThemeClass();
  }

  function applyAccentFromConfig() {
    const c = appConfig?.gui?.accent_color || "cyan";
    document.documentElement.setAttribute("data-accent", c);
  }

  function toggleTheme() {
    darkMode = !darkMode;
    appConfig.gui.theme = darkMode ? "dark" : "light";
    localStorage.setItem("theme", appConfig.gui.theme);
    updateThemeClass();
    saveSettings();
  }

  function openHelp(tabKey) {
    activeHelpTab = tabKey;
  }

  function closeHelp() {
    activeHelpTab = null;
  }

  function dismissTip(tabKey) {
    dismissedTips[tabKey] = true;
    localStorage.setItem("dismissed_tips", JSON.stringify(dismissedTips));
  }

  function updateThemeClass() {
    if (darkMode) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }

  async function loadSettings() {
    try {
      const cfg = await invokeTauri("get_app_settings");
      if (cfg && cfg.gui) {
        if (!["ambos", "arrastrar", "botones"].includes(cfg.gui.file_selection_mode)) {
          cfg.gui.file_selection_mode = "ambos";
        }
        if (!["tray", "taskbar", "close"].includes(cfg.gui.close_action)) {
          cfg.gui.close_action = cfg.gui.close_to_tray === false ? "close" : "tray";
        }
        cfg.gui.close_to_tray = (cfg.gui.close_action === "tray");
        if (!cfg.gui.output_suffix) {
          cfg.gui.output_suffix = "_obfuscated";
        }
        if (!["small", "medium", "large"].includes(cfg.gui.font_size)) {
          cfg.gui.font_size = "medium";
        }
        if (!["system", "dark", "light"].includes(cfg.gui.theme)) {
          cfg.gui.theme = "system";
        }
        if (!["cyan", "emerald", "cobalt", "purple", "amber"].includes(cfg.gui.accent_color)) {
          cfg.gui.accent_color = "cyan";
        }
        appConfig = cfg;
        applyThemeFromConfig();
        applyAccentFromConfig();
        if (cfg.gui.default_tab && cfg.gui.tabs[`show_${cfg.gui.default_tab}`] !== false) {
          activeTab = cfg.gui.default_tab;
        }
      }
    } catch (e) {
      console.warn("Error cargando configuración:", e);
    }
  }

  async function saveSettings() {
    try {
      if (!["ambos", "arrastrar", "botones"].includes(appConfig?.gui?.file_selection_mode)) {
        appConfig.gui.file_selection_mode = "ambos";
      }
      if (!appConfig?.gui?.output_suffix) {
        appConfig.gui.output_suffix = "_obfuscated";
      }
      if (!["small", "medium", "large"].includes(appConfig?.gui?.font_size)) {
        appConfig.gui.font_size = "medium";
      }
      if (!["system", "dark", "light"].includes(appConfig?.gui?.theme)) {
        appConfig.gui.theme = "system";
      }
      if (!["cyan", "emerald", "cobalt", "purple", "amber"].includes(appConfig?.gui?.accent_color)) {
        appConfig.gui.accent_color = "cyan";
      }
      await invokeTauri("save_app_settings", { config: appConfig });
      statusMessage = "✅ Configuración guardada en ~/.lexishield/config.toml";
    } catch (e) {
      statusMessage = "❌ Error al guardar configuración: " + e;
    }
  }

  // Carga de Bóveda Inicial y Desbloqueo
  async function loadInitialVault() {
    try {
      const savedVault = localStorage.getItem("last_vault_path");
      let targetPath = savedVault;

      if (!targetPath) {
        const defaultInfo = await invokeTauri("get_default_vault_info", { password: null });
        if (defaultInfo && defaultInfo.path) {
          targetPath = defaultInfo.path;
        }
      }

      if (targetPath) {
        vaultPath = targetPath;
        pairsSelectedVault = targetPath;
        await checkAndLoadVault(targetPath, null);
      }
    } catch (err) {
      console.warn("No se pudo cargar la bóveda inicial:", err);
    }
  }

  async function checkAndLoadVault(filePath, pwd = null) {
    unlockError = "";
    try {
      const meta = await invokeTauri("inspect_vault_file", { path: filePath, password: pwd });
      if (!meta.exists) {
        isVaultEncrypted = false;
        isVaultUnlocked = false;
        vaultMappings = [];
        initVaultTarget = filePath;
        showInitVaultModal = true;
        statusMessage = `ℹ️ La bóveda '${filePath.split('/').pop()}' no existe. Configura una contraseña para inicializarla.`;
        return;
      }

      isVaultEncrypted = meta.is_encrypted;

      if (meta.is_encrypted && !meta.is_unlocked) {
        unlockTargetVault = filePath;
        isVaultUnlocked = false;
        showUnlockModal = true;
        statusMessage = `🔒 Bóveda cifrada: ${filePath}. Ingrese su contraseña.`;
      } else {
        vaultMappings = meta.items || [];
        isVaultUnlocked = true;
        showUnlockModal = false;
        showInitVaultModal = false;
        unlockPassword = "";
        vaultPath = filePath;
        pairsSelectedVault = filePath;
        localStorage.setItem("last_vault_path", filePath);
        statusMessage = `🔓 Bóveda cargada: ${filePath} (${vaultMappings.length} mapeos)`;
        refreshVaultsList();
      }
    } catch (err) {
      unlockError = String(err);
      statusMessage = "❌ " + err;
      setTimeout(() => {
        const el = document.getElementById("unlock-pwd-input");
        if (el) {
          el.focus();
          if (typeof el.select === "function") el.select();
        }
      }, 50);
    }
  }

  async function handleInitVaultSubmit() {
    const pwd = initVaultPassword.trim();
    if (!pwd) {
      alert("⚠️ Es obligatorio proporcionar una contraseña para inicializar y proteger la bóveda.");
      setTimeout(() => {
        const el = document.getElementById("init-vault-pwd");
        if (el) {
          el.focus();
          if (typeof el.select === "function") el.select();
        }
      }, 50);
      return;
    }
    try {
      const targetName = initVaultTarget ? initVaultTarget.split('/').pop() : "default.lexi";
      const created = await invokeTauri("create_vault_file", {
        name: targetName,
        password: pwd
      });
      showInitVaultModal = false;
      password = pwd;
      initVaultPassword = "";
      statusMessage = `✅ Bóveda '${targetName}' inicializada y cifrada con éxito.`;
      await refreshVaultsList();
      await checkAndLoadVault(created, pwd);
    } catch (e) {
      alert("Error al inicializar bóveda: " + e);
      setTimeout(() => {
        const el = document.getElementById("init-vault-pwd");
        if (el) {
          el.focus();
          if (typeof el.select === "function") el.select();
        }
      }, 50);
    }
  }

  async function handleUnlockVaultSubmit() {
    if (!unlockPassword) {
      unlockError = "Por favor ingrese la contraseña de la bóveda";
      setTimeout(() => {
        const el = document.getElementById("unlock-pwd-input");
        if (el) {
          el.focus();
          if (typeof el.select === "function") el.select();
        }
      }, 50);
      return;
    }
    password = unlockPassword;
    await checkAndLoadVault(unlockTargetVault || vaultPath, unlockPassword);
  }

  function handleCancelUnlock() {
    showUnlockModal = false;
    unlockPassword = "";
    unlockError = "";
  }

  async function openVaultFile() {
    try {
      const selected = await open({
        filters: [{ name: "Bóvedas LexiShield", extensions: ["lexi", "json"] }],
        multiple: false
      });
      if (selected) {
        vaultPath = selected;
        password = "";
        await checkAndLoadVault(selected, null);
      }
    } catch (err) {
      statusMessage = "Error al abrir bóveda: " + err;
    }
  }

  // Operaciones del Studio
  async function handleObfuscate() {
    if (!inputText.trim()) {
      statusMessage = "Por favor ingresa texto en el panel 'Texto claro' para proteger.";
      return;
    }
    loading = true;
    try {
      const res = await invokeTauri("obfuscate_content", {
        text: inputText,
        formatStr: selectedFormat,
        vaultPath: vaultPath || null,
        password: password || null
      });
      outputText = res.obfuscated_text;
      lastReport = res.report;
      vaultMappings = res.mappings || [];
      studioDirection = "obfuscate";
      statusMessage = `✅ Datos protegidos exitosamente: ${res.report.replacements_applied} tokens seudonimizados en ${res.report.elapsed_ms.toFixed(2)}ms`;
      if (autoCopyToClipboard && res.obfuscated_text) {
        try {
          await navigator.clipboard.writeText(res.obfuscated_text);
          copiedOutputFlash = true;
          setTimeout(() => {
            copiedOutputFlash = false;
          }, 2000);
          statusMessage = `✅ Datos protegidos y copiados al portapapeles: ${res.report.replacements_applied} tokens en ${res.report.elapsed_ms.toFixed(2)}ms`;
        } catch (e) {
          console.warn("Auto-copy error:", e);
        }
      }
      pushToSessionHistory("obfuscate", res.report);
      actionResultModal = {
        title: "Protección Finalizada con Éxito",
        type: "success",
        message: `El texto ha sido transformado para proteger tus datos confidenciales antes de enviarlo a la IA o compartirlo con terceros.`,
        details: [
          { label: "Tokens Seudonimizados", value: `${res.report.replacements_applied}` },
          { label: "Tiempo de Procesamiento", value: `${res.report.elapsed_ms.toFixed(2)} ms` },
          { label: "Bóveda Destino", value: vaultPath ? vaultPath.split('/').pop() : "Predeterminada" }
        ]
      };
      refreshVaultsList();
    } catch (err) {
      statusMessage = "Error al proteger datos: " + err;
    } finally {
      loading = false;
    }
  }

  async function handleDeobfuscate() {
    if (!outputText.trim()) {
      statusMessage = "Por favor ingresa texto en el panel 'Texto protegido' para restaurar.";
      return;
    }
    loading = true;
    try {
      const res = await invokeTauri("deobfuscate_content", {
        text: outputText,
        formatStr: selectedFormat,
        vaultPath: vaultPath || null,
        password: password || null,
        customMappings: null
      });
      inputText = res.deobfuscated_text;
      lastReport = res.report;
      studioDirection = "deobfuscate";
      deobfuscateSuccessFlash = true;
      setTimeout(() => {
        deobfuscateSuccessFlash = false;
      }, 2000);
      statusMessage = `✅ Datos restaurados exitosamente: ${res.report.replacements_applied} tokens recuperados en ${res.report.elapsed_ms.toFixed(2)}ms`;
      if (autoCopyToClipboard && res.deobfuscated_text) {
        try {
          await navigator.clipboard.writeText(res.deobfuscated_text);
          copiedInputFlash = true;
          setTimeout(() => {
            copiedInputFlash = false;
          }, 2000);
          statusMessage = `✅ Datos restaurados y copiados al portapapeles: ${res.report.replacements_applied} tokens en ${res.report.elapsed_ms.toFixed(2)}ms`;
        } catch (e) {
          console.warn("Auto-copy error:", e);
        }
      }
      pushToSessionHistory("deobfuscate", res.report);
      actionResultModal = {
        title: "Restauración Finalizada con Éxito",
        type: "success",
        message: "Las respuestas de la IA han sido procesadas recuperando los valores reales originales mediante tu bóveda local.",
        details: [
          { label: "Tokens Restaurados", value: `${res.report.replacements_applied}` },
          { label: "Tiempo de Proceso", value: `${res.report.elapsed_ms.toFixed(2)} ms` },
          { label: "Bóveda Utilizada", value: vaultPath ? vaultPath.split('/').pop() : "Predeterminada" }
        ]
      };
    } catch (err) {
      statusMessage = "Error al restaurar datos: " + err;
    } finally {
      loading = false;
    }
  }

  async function processFilePathInStudio(selected, reverse = false, customOptions = null) {
    if (!selected) return;
    try {
      loading = true;
      const useCopy = customOptions ? customOptions.copyMode : copyMode;
      const activeSuffix = customOptions?.suffix || (reverse ? ".desofuscado" : copySuffix);
      const useMd = customOptions ? (hasPandoc && customOptions.exportMarkdown) : (hasPandoc && exportMarkdown);

      let outPath = selected;
      if (useCopy) {
        const dotIdx = selected.lastIndexOf(".");
        if (dotIdx !== -1) {
          outPath = selected.substring(0, dotIdx) + activeSuffix + selected.substring(dotIdx);
        } else {
          outPath = selected + activeSuffix;
        }
      }

      const rep = await invokeTauri("process_file_command", {
        inputPath: selected,
        outputPath: outPath,
        formatStr: selectedFormat,
        vaultPath: vaultPath || null,
        password: password || null,
        reverse: reverse,
        exportMd: useMd
      });
      lastReport = rep;
      const actionName = reverse ? "restaurado" : "protegido";
      statusMessage = `✅ Archivo ${actionName} (${rep.replacements_applied} tokens procesados). Guardado en: ${outPath}`;
      actionResultModal = {
        title: reverse ? "Archivo Restaurado con Éxito" : "Archivo Protegido con Éxito",
        type: "success",
        message: reverse
          ? `El archivo ha sido restaurado reconstruyendo los datos originales mediante tu bóveda local.`
          : `El archivo ha sido procesado y guardado con los tokens confidenciales protegidos.`,
        details: [
          { label: "Archivo Origen", value: selected.split(/[/\\]/).pop() },
          { label: "Archivo Guardado", value: outPath.split(/[/\\]/).pop() },
          { label: reverse ? "Tokens Restaurados" : "Tokens Seudonimizados", value: `${rep.replacements_applied}` },
          { label: "Tiempo", value: `${rep.elapsed_ms.toFixed(2)} ms` },
          { label: "Exportado a Markdown", value: useMd ? "Sí (.md)" : "No" }
        ]
      };
      refreshVaultsList();
    } catch (err) {
      statusMessage = `Error al ${reverse ? "restaurar" : "proteger"} archivo: ` + err;
    } finally {
      loading = false;
    }
  }

  async function openFileToProcess(reverse = false) {
    try {
      const selected = await open({
        filters: [
          { name: "Documentos y Logs", extensions: ["txt", "log", "json", "xml", "csv", "tsv", "docx", "xlsx", "pptx", "md", "markdown"] },
          { name: "Todos los archivos", extensions: ["*"] }
        ],
        multiple: false
      });
      if (selected) {
        await processFilePathInStudio(selected, reverse);
      }
    } catch (err) {
      statusMessage = "Error al abrir archivo: " + err;
    }
  }

  // Operaciones del Escáner y Drag & Drop
  async function auditFilePath(filePath) {
    if (!filePath) return;
    try {
      loading = true;
      detectedItems = [];
      auditSummary = null;
      activeTab = "studio";
      studioSubView = "audit";
      const res = await invokeTauri("scan_file_command", {
        path: filePath,
        vaultPath: pairsSelectedVault || vaultPath || null,
        password: password || null
      });
      detectedItems = res.items || [];
      auditSummary = res;
      statusMessage = `🔍 Archivo escaneado: ${res.total_entities} tokens detectados en 1 archivo.`;
      actionResultModal = {
        title: "Escaneo de Archivo Finalizado",
        type: "info",
        message: `El análisis del archivo ha finalizado satisfactoriamente sin modificar su contenido original.`,
        details: [
          { label: "Archivo Inspeccionado", value: filePath.split('/').pop() },
          { label: "Tokens Confidenciales", value: `${res.total_entities}` },
          { label: "Falsos Positivos Omitidos", value: `${(res.items || []).filter(i => i.omitted).length}` }
        ]
      };
    } catch (err) {
      statusMessage = "Error al escanear archivo: " + err;
    } finally {
      loading = false;
    }
  }

  async function openFileToAudit() {
    try {
      const selected = await open({
        filters: [{ name: "Archivos de texto y logs", extensions: ["*"] }],
        multiple: false
      });
      if (selected) {
        await auditFilePath(selected);
      }
    } catch (err) {
      statusMessage = "Error al abrir archivo: " + err;
    }
  }

  async function auditDirectoryPath(dirPath) {
    if (!dirPath) return;
    try {
      loading = true;
      isScanningDirectory = true;
      detectedItems = [];
      auditSummary = null;
      activeTab = "studio";
      studioSubView = "audit";
      scanCurrentFile = "Iniciando recorrido...";
      scanFilesCount = 0;
      scanFilesIgnored = 0;
      scanEntitiesCount = 0;
      statusMessage = "Escaneando carpeta: " + dirPath;

      const res = await invokeTauri("scan_directory_command", {
        dirPath,
        recursive: recursiveScan,
        noGitIgnore: noGitIgnoreScan,
        vaultPath: pairsSelectedVault || vaultPath || null,
        password: password || null
      });
      auditSummary = res;
      statusMessage = `🔍 Directorio escaneado: ${res.files_scanned} analizados, ${res.files_ignored} omitidos, ${res.total_entities} tokens detectados.`;
      actionResultModal = {
        title: "Escaneo de Directorio Finalizado",
        type: "info",
        message: `El recorrido del directorio ha concluido con éxito. Puedes revisar o activar tokens en la tabla inferior.`,
        details: [
          { label: "Directorio", value: dirPath.split('/').pop() },
          { label: "Archivos Analizados", value: `${res.files_scanned}` },
          { label: "Archivos Omitidos", value: `${res.files_ignored}` },
          { label: "Tokens Confidenciales", value: `${res.total_entities}` }
        ]
      };
    } catch (err) {
      statusMessage = "Error al escanear carpeta: " + err;
    } finally {
      loading = false;
      isScanningDirectory = false;
    }
  }

  async function openDirectoryToAudit() {
    try {
      const selected = await open({
        directory: true,
        multiple: false
      });
      if (selected) {
        await auditDirectoryPath(selected);
      }
    } catch (err) {
      statusMessage = "Error al abrir carpeta: " + err;
    }
  }

  async function scanCurrentStudioText() {
    const textToScan = inputText.trim() || outputText.trim() || textToAudit.trim();
    if (!textToScan) {
      statusMessage = "Por favor ingresa o pega texto en los paneles para auditar.";
      return;
    }
    try {
      loading = true;
      detectedItems = [];
      auditSummary = null;
      studioSubView = "audit";

      const res = await invokeTauri("scan_content", {
        content: textToScan,
        vaultPath: pairsSelectedVault || vaultPath || null,
        password: password || null
      });

      detectedItems = (res.items || []).map(item => ({
        ...item,
        file: "[Texto del Studio]"
      }));

      auditSummary = {
        files_scanned: 1,
        files_ignored: 0,
        total_entities: res.total_entities,
        vault_used: pairsSelectedVault || "Sesión"
      };

      if (res.total_entities === 0) {
        statusMessage = "🔍 Análisis finalizado: No se identificaron tokens confidenciales en el texto.";
      } else {
        statusMessage = `🔍 Texto analizado: ${res.total_entities} tokens confidenciales detectados y clasificados en la tabla.`;
      }
    } catch (err) {
      statusMessage = "Error al analizar texto: " + err;
      actionResultModal = {
        title: "Error al Analizar Texto",
        type: "info",
        message: `No se pudo analizar el texto proporcionado: ${err}`,
        details: []
      };
    } finally {
      loading = false;
    }
  }

  async function scanPastedText() {
    if (!textToAudit.trim()) return;
    try {
      loading = true;
      detectedItems = [];
      auditSummary = null;
      activeTab = "studio";
      studioSubView = "audit";

      const res = await invokeTauri("scan_content", {
        content: textToAudit,
        vaultPath: pairsSelectedVault || vaultPath || null,
        password: password || null
      });

      detectedItems = (res.items || []).map(item => ({
        ...item,
        file: "[Texto copiado]"
      }));

      auditSummary = {
        files_scanned: 1,
        files_ignored: 0,
        total_entities: res.total_entities,
        vault_used: pairsSelectedVault || "Sesión"
      };

      if (res.total_entities === 0) {
        statusMessage = "🔍 Análisis finalizado: No se identificaron tokens confidenciales en el fragmento.";
        showTextScanPanel = true;
      } else {
        statusMessage = `🔍 Texto analizado: ${res.total_entities} tokens confidenciales detectados y listados abajo.`;
        showTextScanPanel = false;
      }
    } catch (err) {
      statusMessage = "Error al analizar texto: " + err;
      actionResultModal = {
        title: "Error al Analizar Texto",
        type: "info",
        message: `No se pudo analizar el texto proporcionado: ${err}`,
        details: []
      };
    } finally {
      loading = false;
    }
  }

  function clearScanResults() {
    detectedItems = [];
    auditSummary = null;
    selectedScanOriginals = [];
    textToAudit = "";
    statusMessage = "Auditoría reiniciada. Listo para un nuevo análisis.";
  }

  function sendPastedTextToStudio() {
    if (!textToAudit.trim()) return;
    inputText = textToAudit;
    textToAudit = "";
    detectedItems = [];
    auditSummary = null;
    selectedScanOriginals = [];
    activeTab = "studio";
    studioSubView = "transform";
    statusMessage = "📋 Texto transferido al editor de Studio.";
  }

  async function handleDroppedPath(droppedPath, targetPanel = "deobfuscated") {
    if (!droppedPath) return;
    try {
      const isDir = await invokeTauri("is_directory", { path: droppedPath });
      const itemName = droppedPath.split(/[/\\]/).filter(Boolean).pop() || droppedPath;
      const isReverse = (targetPanel === "obfuscated");
      const defaultSuffix = isReverse ? ".desofuscado" : copySuffix;
      studioDropModal = {
        path: droppedPath,
        name: itemName,
        isDir: isDir,
        isReverse: isReverse,
        copyMode: true,
        suffix: defaultSuffix,
        saveSuffixDefault: false,
        cleanCache: true,
        exportMarkdown: hasPandoc && exportMarkdown
      };
    } catch (err) {
      statusMessage = "Error al procesar elemento arrastrado: " + err;
    }
  }

  function computeOutputPreview(modal) {
    if (!modal) return "";
    if (modal.isDir) {
      if (!modal.copyMode) return `${modal.name} (Directorio original in-situ)`;
      return `${modal.name}${modal.suffix || ''}/ (Copias dentro de la carpeta)`;
    }
    if (!modal.copyMode) return `${modal.name} (Sobrescribir archivo original)`;
    const sfx = modal.suffix || "";
    const dotIdx = modal.name.lastIndexOf(".");
    if (dotIdx !== -1) {
      return modal.name.substring(0, dotIdx) + sfx + modal.name.substring(dotIdx);
    }
    return modal.name + sfx;
  }

  async function confirmStudioDropModal() {
    if (!studioDropModal) return;
    const { path, isDir, isReverse, copyMode, suffix, saveSuffixDefault, cleanCache, exportMarkdown: expMd } = studioDropModal;
    studioDropModal = null;

    try {
      if (saveSuffixDefault && suffix) {
        copySuffix = suffix;
        if (appConfig?.gui) {
          appConfig.gui.output_suffix = suffix;
          await saveSettings();
        }
      }

      if (isDir) {
        if (cleanCache) {
          await invokeTauri("clear_scan_cache_command");
          statusMessage = "🧹 Cachés de sesión limpiadas antes de analizar el directorio.";
        }
        activeTab = "studio";
        studioSubView = "audit";
        await auditDirectoryPath(path);
        statusMessage = `📁 Directorio cargado para análisis y ${isReverse ? 'restauración' : 'protección'}: ${path}`;
      } else {
        await processFilePathInStudio(path, isReverse, {
          copyMode,
          suffix,
          exportMarkdown: expMd
        });
      }
    } catch (err) {
      statusMessage = "Error al procesar elemento: " + err;
    }
  }

  function cancelStudioDropModal() {
    studioDropModal = null;
  }

  async function cancelScan() {
    try {
      await invokeTauri("cancel_scan_command");
      statusMessage = "Cancelando escaneo...";
    } catch (err) {
      console.warn("Error al cancelar escaneo:", err);
    }
  }

  // Selección y Omisión en la Pestaña Escáner
  let selectedScanOriginals = [];

  function toggleSelectAllScanItems() {
    if (selectedScanOriginals.length === detectedItems.length) {
      selectedScanOriginals = [];
    } else {
      selectedScanOriginals = detectedItems.map(it => it.original);
    }
  }

  function handleBatchOmitScanItems(shouldOmit) {
    if (selectedScanOriginals.length === 0) {
      detectedItems = detectedItems.map(it => {
        return {
          ...it,
          omitted: shouldOmit,
          pseudonym: shouldOmit ? "=" : (it.pseudonym === "=" ? `[TOKEN_${Math.floor(0x1000 + Math.random() * 0xEFFF).toString(16).toUpperCase()}]` : it.pseudonym)
        };
      });
      statusMessage = shouldOmit
        ? "🚫 Todas las entidades marcadas para OMITIR (no se ofuscarán)."
        : "🛡️ Todas las entidades reactivadas para ofuscación.";
    } else {
      detectedItems = detectedItems.map(it => {
        if (selectedScanOriginals.includes(it.original)) {
          return {
            ...it,
            omitted: shouldOmit,
            pseudonym: shouldOmit ? "=" : (it.pseudonym === "=" ? `[TOKEN_${Math.floor(0x1000 + Math.random() * 0xEFFF).toString(16).toUpperCase()}]` : it.pseudonym)
          };
        }
        return it;
      });
      statusMessage = shouldOmit
        ? `🚫 ${selectedScanOriginals.length} entidades seleccionadas marcadas para OMITIR.`
        : `🛡️ ${selectedScanOriginals.length} entidades seleccionadas reactivadas para ofuscación.`;
      selectedScanOriginals = [];
    }
  }

  function toggleSingleScanOmit(item) {
    item.omitted = !item.omitted;
    if (item.omitted) {
      item.pseudonym = "=";
    } else if (item.pseudonym === "=") {
      item.pseudonym = `[TOKEN_${Math.floor(0x1000 + Math.random() * 0xEFFF).toString(16).toUpperCase()}]`;
    }
    detectedItems = [...detectedItems];
  }

  async function handleSaveScanToVault() {
    if (detectedItems.length === 0) return;
    const targetVault = pairsSelectedVault || vaultPath || 'default.lexi';
    try {
      const itemsToSave = detectedItems.map(it => ({
        original: it.original,
        pseudonym: it.omitted ? "=" : it.pseudonym,
        detector_type: it.detector_type,
        omitted: !!it.omitted
      }));

      let existingItems = [];
      try {
        const meta = await invokeTauri("inspect_vault_file", { path: targetVault, password: password || null });
        if (meta && meta.items) {
          existingItems = meta.items;
        }
      } catch (_) {}

      const combinedMap = new Map();
      for (const it of existingItems) {
        combinedMap.set(it.original, it);
      }
      for (const it of itemsToSave) {
        combinedMap.set(it.original, it);
      }

      const mergedList = Array.from(combinedMap.values());
      await invokeTauri("save_vault_file", {
        path: targetVault,
        password: password || null,
        items: mergedList
      });

      vaultMappings = mergedList;
      statusMessage = `💾 ${itemsToSave.length} entidades del escaneo guardadas e integradas en '${targetVault.split('/').pop()}'.`;
      
      // Limpiar de la pantalla los elementos guardados
      const savedOriginals = new Set(itemsToSave.map(it => it.original));
      detectedItems = detectedItems.filter(it => !savedOriginals.has(it.original));
      selectedScanOriginals = []; // Limpiar selección

      await refreshVaultsList();
    } catch (e) {
      alert("Error al guardar mapeos en la bóveda: " + e);
    }
  }

  // Operaciones de la Pestaña Pares
  async function loadPairsForVault(path) {
    if (!path) return;
    try {
      pairsSelectedVault = path;
      const meta = await invokeTauri("inspect_vault_file", { path, password: password || null });
      if (meta.is_encrypted && !meta.is_unlocked) {
        unlockTargetVault = path;
        showUnlockModal = true;
      } else {
        vaultMappings = meta.items || [];
      }
    } catch (e) {
      statusMessage = "Error al cargar pares de la bóveda: " + e;
    }
  }

  async function handleAddPairSubmit() {
    if (!newPairOriginal.trim() || !newPairPseudo.trim()) {
      alert("Por favor introduce el valor original y el seudónimo.");
      return;
    }
    try {
      const updated = await invokeTauri("update_mapping_pair", {
        vaultPath: pairsSelectedVault || vaultPath,
        password: password || null,
        original: newPairOriginal.trim(),
        pseudonym: newPairPseudo.trim(),
        detectorType: newPairType
      });
      vaultMappings = updated;
      showAddPairModal = false;
      newPairOriginal = "";
      newPairPseudo = "";
      statusMessage = "✅ Par agregado con éxito a la bóveda.";
    } catch (e) {
      alert("Error al agregar par: " + e);
    }
  }

  function startInlineEditPair(item) {
    editingPairOriginal = item.original;
    editingPairPseudo = item.pseudonym;
  }

  async function saveInlineEditPair(item) {
    try {
      const updated = await invokeTauri("update_mapping_pair", {
        vaultPath: pairsSelectedVault || vaultPath,
        password: password || null,
        original: item.original,
        pseudonym: editingPairPseudo.trim(),
        detectorType: item.detector_type
      });
      vaultMappings = updated;
      editingPairOriginal = null;
      statusMessage = "✅ Par actualizado.";
    } catch (e) {
      alert("Error al actualizar par: " + e);
    }
  }

  async function deleteSinglePair(original) {
    if (!confirm(`¿Eliminar el mapeo para '${original}'?`)) return;
    try {
      const updated = await invokeTauri("delete_mapping_pairs", {
        vaultPath: pairsSelectedVault || vaultPath,
        password: password || null,
        originals: [original]
      });
      vaultMappings = updated;
      statusMessage = `✅ Mapeo '${original}' eliminado.`;
    } catch (e) {
      alert("Error al eliminar par: " + e);
    }
  }

  async function handleToggleOmitPair(item) {
    const isCurrentlyOmitted = item.omitted || item.pseudonym === "=";
    const newPseudo = isCurrentlyOmitted ? `[TOKEN_${Math.floor(0x1000 + Math.random() * 0xEFFF).toString(16).toUpperCase()}]` : "=";
    try {
      const updated = await invokeTauri("update_mapping_pair", {
        vaultPath: pairsSelectedVault || vaultPath,
        password: password || null,
        original: item.original,
        pseudonym: newPseudo,
        detectorType: item.detector_type
      });
      vaultMappings = updated;
      statusMessage = isCurrentlyOmitted
        ? `✅ Entidad '${item.original}' reactivada para ofuscación.`
        : `🚫 Entidad '${item.original}' configurada como ignorada/omitida.`;
    } catch (e) {
      alert("Error al cambiar estado de omisión: " + e);
    }
  }

  async function handleBatchOmitPairs(shouldOmit) {
    if (selectedPairKeys.length === 0) return;
    try {
      let currentList = [...vaultMappings];
      for (const orig of selectedPairKeys) {
        const found = currentList.find(m => m.original === orig);
        const newPseudo = shouldOmit ? "=" : `[TOKEN_${Math.floor(0x1000 + Math.random() * 0xEFFF).toString(16).toUpperCase()}]`;
        currentList = await invokeTauri("update_mapping_pair", {
          vaultPath: pairsSelectedVault || vaultPath,
          password: password || null,
          original: orig,
          pseudonym: newPseudo,
          detectorType: found?.detector_type || null
        });
      }
      vaultMappings = currentList;
      selectedPairKeys = [];
      statusMessage = shouldOmit
        ? `🚫 Entidades seleccionadas configuradas como ignoradas/omitidas.`
        : `✓ Entidades seleccionadas reactivadas para ofuscación.`;
    } catch (e) {
      alert("Error al actualizar omisiones en lote: " + e);
    }
  }

  async function deleteSelectedPairs() {
    if (selectedPairKeys.length === 0) return;
    if (!confirm(`¿Eliminar los ${selectedPairKeys.length} pares seleccionados?`)) return;
    try {
      const updated = await invokeTauri("delete_mapping_pairs", {
        vaultPath: pairsSelectedVault || vaultPath,
        password: password || null,
        originals: selectedPairKeys
      });
      vaultMappings = updated;
      selectedPairKeys = [];
      statusMessage = "✅ Pares seleccionados eliminados con éxito.";
    } catch (e) {
      alert("Error al eliminar pares en lote: " + e);
    }
  }

  function toggleSelectAllPairs() {
    if (selectedPairKeys.length === filteredPairs.length) {
      selectedPairKeys = [];
    } else {
      selectedPairKeys = filteredPairs.map(p => p.original);
    }
  }

  $: filteredPairs = vaultMappings.filter(item => {
    const matchesSearch = !pairSearch ||
      item.original.toLowerCase().includes(pairSearch.toLowerCase()) ||
      item.pseudonym.toLowerCase().includes(pairSearch.toLowerCase());
    const matchesType = pairTypeFilter === "all" || item.detector_type === pairTypeFilter;
    return matchesSearch && matchesType;
  });

  // Operaciones de la Pestaña Bóvedas y Backups
  async function refreshVaultsList() {
    try {
      allVaultsList = await invokeTauri("list_all_vaults");
    } catch (e) {
      console.warn("Error listando bóvedas:", e);
    }
  }

  async function handleCreateVaultSubmit() {
    if (!newVaultName.trim()) {
      alert("Introduce un nombre para la nueva bóveda.");
      setTimeout(() => {
        const el = document.getElementById("create-vault-name-input");
        if (el) {
          el.focus();
          if (typeof el.select === "function") el.select();
        }
      }, 50);
      return;
    }
    const pwd = newVaultPassword.trim();
    if (!pwd) {
      alert("⚠️ Es obligatorio configurar una contraseña para proteger la nueva bóveda.");
      setTimeout(() => {
        const el = document.getElementById("create-vault-pwd-input");
        if (el) {
          el.focus();
          if (typeof el.select === "function") el.select();
        }
      }, 50);
      return;
    }
    try {
      const createdPath = await invokeTauri("create_vault_file", {
        name: newVaultName.trim(),
        password: pwd
      });
      showCreateVaultModal = false;
      newVaultName = "";
      newVaultPassword = "";
      statusMessage = `✅ Bóveda creada y cifrada: ${createdPath}`;
      await refreshVaultsList();
      await checkAndLoadVault(createdPath, pwd);
    } catch (e) {
      alert("Error al crear bóveda: " + e);
    }
  }

  function openRenameVault(vault) {
    renameOldVaultName = vault.name;
    renameNewVaultName = vault.name;
    showRenameVaultModal = true;
  }

  async function handleRenameVaultSubmit() {
    if (!renameNewVaultName.trim() || renameNewVaultName === renameOldVaultName) {
      showRenameVaultModal = false;
      return;
    }
    try {
      const renamedPath = await invokeTauri("rename_vault_file", {
        oldName: renameOldVaultName,
        newName: renameNewVaultName.trim()
      });
      showRenameVaultModal = false;
      statusMessage = `✅ Bóveda renombrada: ${renamedPath}`;
      await refreshVaultsList();
    } catch (e) {
      alert("Error al renombrar: " + e);
    }
  }

  async function handleDeleteVault(vault) {
    if (!confirm(`⚠️ ¿Estás seguro de eliminar permanentemente la bóveda '${vault.name}'?`)) return;
    try {
      await invokeTauri("delete_vault_file", { name: vault.name });
      statusMessage = `✅ Bóveda '${vault.name}' eliminada.`;
      await refreshVaultsList();
    } catch (e) {
      alert("Error al eliminar bóveda: " + e);
    }
  }

  async function handleExportVault(vault) {
    try {
      const dest = await save({
        defaultPath: vault.name,
        filters: [{ name: "Bóveda LexiShield", extensions: ["lexi"] }]
      });
      if (!dest) return;
      await invokeTauri("export_vault_file", { name: vault.name, destPath: dest });
      statusMessage = `✅ Bóveda exportada a: ${dest}`;
    } catch (e) {
      alert("Error al exportar: " + e);
    }
  }

  async function handleImportVault() {
    try {
      const src = await open({
        filters: [{ name: "Archivos de Bóveda", extensions: ["lexi", "json"] }],
        multiple: false
      });
      if (!src) return;
      const importedPath = await invokeTauri("import_vault_file", { srcPath: src, name: null });
      statusMessage = `✅ Bóveda importada con éxito: ${importedPath}`;
      await refreshVaultsList();
    } catch (e) {
      alert("Error al importar: " + e);
    }
  }

  async function handleRestoreVaultBak(vaultName) {
    if (!confirm(`⚠️ ¿Restaurar la bóveda '${vaultName}' desde su última copia de seguridad automática (.bak)? Se recuperará el estado previo y se resolverá cualquier daño en el archivo.`)) return;
    try {
      const restored = await invokeTauri("restore_vault_backup_file", { name: vaultName });
      statusMessage = `✅ Bóveda '${vaultName}' restaurada con éxito desde su copia previa (.bak).`;
      await refreshVaultsList();
      if (vaultPath && vaultPath.endsWith(vaultName)) {
        await checkAndLoadVault(restored, password || null);
      }
    } catch (e) {
      alert("Error al restaurar copia de seguridad previa (.bak): " + e);
    }
  }

  async function handleCreateBackup() {
    try {
      const dest = await save({
        defaultPath: `backup_lexishield_${Date.now()}.tar.bz2`,
        filters: [{ name: "Copia de Seguridad Bzip2", extensions: ["tar.bz2", "lexibak"] }]
      });
      if (!dest) return;
      backupStatus = "Generando copia comprimida con Bzip2...";
      const res = await invokeTauri("create_backup_file", {
        outputPath: dest,
        password: backupPassword.trim() || null
      });
      showBackupModal = false;
      backupPassword = "";
      statusMessage = `📦 Copia de seguridad Bzip2 guardada con éxito en: ${res}`;
    } catch (e) {
      alert("Error al crear copia de seguridad: " + e);
    }
  }

  async function handleRestoreBackup() {
    try {
      const src = await open({
        filters: [{ name: "Copias de Seguridad LexiShield", extensions: ["tar.bz2", "bz2", "lexibak", "tar"] }],
        multiple: false
      });
      if (!src) return;
      if (!confirm(`⚠️ ¿Restaurar copia desde '${src}'? Se sobrescribirán configuraciones y bóvedas con el mismo nombre.`)) return;
      
      let pwd = null;
      // Probar si requiere contraseña
      try {
        const count = await invokeTauri("restore_backup_file", { inputPath: src, password: null });
        statusMessage = `✅ Copia de seguridad restaurada con éxito (${count} archivos recuperados).`;
        await refreshVaultsList();
        await loadSettings();
      } catch (err) {
        if (String(err).includes("contraseña")) {
          pwd = prompt("Introduce la contraseña para descifrar la copia de seguridad:");
          if (pwd !== null) {
            const count = await invokeTauri("restore_backup_file", { inputPath: src, password: pwd });
            statusMessage = `✅ Copia de seguridad descifrada y restaurada (${count} archivos recuperados).`;
            await refreshVaultsList();
            await loadSettings();
          }
        } else {
          alert("Error al restaurar: " + err);
        }
      }
    } catch (e) {
      alert("Error al restaurar copia: " + e);
    }
  }

  // Operaciones de Portapapeles
  async function handleClearClipboard() {
    try {
      await invokeTauri("clear_system_clipboard");
      statusMessage = "🧹 Portapapeles del sistema vaciado con éxito.";
    } catch (e) {
      statusMessage = "Error al vaciar portapapeles: " + e;
    }
  }

  function toggleWatchMode() {
    isWatching = !isWatching;
    if (isWatching) {
      statusMessage = `👀 Monitorización del portapapeles activada (${watchDirection === 'obfuscate' ? 'Ofuscación' : 'Desofuscación'}).`;
      startWatchPolling();
    } else {
      if (watchPollInterval) clearInterval(watchPollInterval);
      watchPollInterval = null;
      statusMessage = "⏸️ Monitorización del portapapeles detenida.";
    }
  }

  function startWatchPolling() {
    if (watchPollInterval) clearInterval(watchPollInterval);
    watchPollInterval = setInterval(async () => {
      try {
        const text = await invokeTauri("get_system_clipboard");
        if (clipboardError) clipboardError = null;
        if (text && text.trim() && text !== lastSeenClipboard) {
          lastSeenClipboard = text;
          if (watchDirection === "obfuscate") {
            const res = await invokeTauri("obfuscate_content", {
              text,
              formatStr: "auto",
              vaultPath: vaultPath || null,
              password: password || null
            });
            if (res.obfuscated_text !== text) {
              lastSeenClipboard = res.obfuscated_text;
              await invokeTauri("set_system_clipboard", { text: res.obfuscated_text });
              
              // Extraer tokens detectados
              const detectedTokens = (res.mappings || []).filter(m => text.includes(m.original));
              clipboardEvents = [
                {
                  id: Date.now() + Math.random(),
                  timestamp: new Date().toLocaleTimeString(),
                  direction: "obfuscate",
                  originalText: text,
                  transformedText: res.obfuscated_text,
                  tokens: detectedTokens,
                  count: res.report?.replacements_applied || detectedTokens.length
                },
                ...clipboardEvents.slice(0, 49)
              ];

              triggerClipboardNotification(`Portapapeles ofuscado (${detectedTokens.length} tokens detectados)`);
              scheduleAutoClearIfNeeded();
            } else {
              // Texto copiado sin tokens confidenciales reconocidos
              clipboardEvents = [
                {
                  id: Date.now() + Math.random(),
                  timestamp: new Date().toLocaleTimeString(),
                  direction: "obfuscate",
                  originalText: text,
                  transformedText: text,
                  tokens: [],
                  count: 0
                },
                ...clipboardEvents.slice(0, 49)
              ];
              triggerClipboardNotification("Portapapeles verificado: 0 tokens reconocidos");
            }
          } else {
            const res = await invokeTauri("deobfuscate_content", {
              text,
              formatStr: "auto",
              vaultPath: vaultPath || null,
              password: password || null,
              customMappings: null
            });
            if (res.deobfuscated_text !== text) {
              lastSeenClipboard = res.deobfuscated_text;
              await invokeTauri("set_system_clipboard", { text: res.deobfuscated_text });
              
              const detectedTokens = (res.mappings || []).filter(m => text.includes(m.pseudonym));
              clipboardEvents = [
                {
                  id: Date.now() + Math.random(),
                  timestamp: new Date().toLocaleTimeString(),
                  direction: "deobfuscate",
                  originalText: text,
                  transformedText: res.deobfuscated_text,
                  tokens: detectedTokens,
                  count: res.report?.replacements_applied || detectedTokens.length
                },
                ...clipboardEvents.slice(0, 49)
              ];

              triggerClipboardNotification(`Portapapeles desofuscado (${detectedTokens.length} tokens restaurados)`);
              scheduleAutoClearIfNeeded();
            } else {
              // Texto copiado sin tokens seudónimos para revertir
              clipboardEvents = [
                {
                  id: Date.now() + Math.random(),
                  timestamp: new Date().toLocaleTimeString(),
                  direction: "deobfuscate",
                  originalText: text,
                  transformedText: text,
                  tokens: [],
                  count: 0
                },
                ...clipboardEvents.slice(0, 49)
              ];
              triggerClipboardNotification("Portapapeles verificado: 0 tokens reconocidos");
            }
          }
        }
      } catch (e) {
        clipboardError = String(e);
        statusMessage = "⚠️ Portapapeles: " + String(e);
      }
    }, 300);
  }

  function triggerClipboardNotification(msg) {
    const notifyType = appConfig?.gui?.clipboard?.notification_type || "toast";
    if (notifyType === "toast") {
      statusMessage = `📋 ${msg}`;
    } else if (notifyType === "visual_flash") {
      clipboardFlash = true;
      setTimeout(() => { clipboardFlash = false; }, 400);
    }
  }

  function scheduleAutoClearIfNeeded() {
    if (appConfig?.gui?.clipboard?.auto_clear_enabled) {
      const secs = appConfig?.gui?.clipboard?.auto_clear_seconds || 5;
      if (autoClearTimer) clearTimeout(autoClearTimer);
      autoClearTimer = setTimeout(async () => {
        await handleClearClipboard();
      }, secs * 1000);
    }
  }

  // Operaciones de Depuración
  async function toggleDebugMode() {
    try {
      debugMode = await invokeTauri("set_debug_mode_command", { enabled: !debugMode });
      statusMessage = debugMode ? "🐞 Modo DEBUG activado." : "ℹ️ Modo DEBUG desactivado.";
      if (debugMode) fetchDebugLogs();
    } catch (e) {
      statusMessage = "Error al cambiar modo debug: " + e;
    }
  }

  async function fetchDebugLogs() {
    try {
      debugLogs = await invokeTauri("get_debug_logs", { limit: 500 });
    } catch (e) {}
  }

  async function clearDebugLogs() {
    try {
      await invokeTauri("clear_debug_logs");
      debugLogs = [];
      statusMessage = "🧹 Registros limpiados.";
    } catch (e) {}
  }

  function copyDebugLogs() {
    if (debugLogs.length === 0) return;
    const text = debugLogs.map(l => `[${l.timestamp} ${l.level} ${l.target}] ${l.message}`).join("\n");
    navigator.clipboard.writeText(text);
    statusMessage = "📋 Registros copiados al portapapeles.";
  }

  function openLogsModal() {
    showLogsModal = true;
    fetchDebugLogs();
    if (!logPollingInterval) {
      logPollingInterval = setInterval(fetchDebugLogs, 1500);
    }
  }

  function closeLogsModal() {
    showLogsModal = false;
    if (logPollingInterval) {
      clearInterval(logPollingInterval);
      logPollingInterval = null;
    }
  }

  // Cálculo Dinámico del Comando CLI Equivalente en Vivo
  $: currentCliCommand = (() => {
    const vaultArg = vaultPath ? ` -m "${vaultPath}"` : "";
    const cleanArg = cleanCache ? " --clean-cache" : "";
    const copyArg = copyMode ? ` --copy --suffix "${copySuffix}"` : "";
    const pwdArg = password ? ` -p "***"` : "";

    if (activeTab === "studio") {
      if (studioSubView === "audit") {
        const recArg = recursiveScan ? " -r" : " --no-recursive";
        const noGitArg = noGitIgnoreScan ? " --no-git-ignore" : "";
        return `lexishield scan -i "<ruta>"${recArg}${noGitArg}${vaultArg}`;
      }
      return `lexishield obfuscate -f ${selectedFormat}${vaultArg}${cleanArg}${copyArg}${pwdArg}`;
    } else if (activeTab === "pairs") {
      return `lexishield dict list${vaultArg}`;
    } else if (activeTab === "vaults") {
      return `lexishield vault list`;
    } else if (activeTab === "watch") {
      return `lexishield watch -d ${watchDirection} -f ${selectedFormat}${vaultArg}`;
    } else if (activeTab === "config") {
      return `lexishield config show`;
    }
    return "lexishield --help";
  })();

  function copyCurrentCliCommand() {
    navigator.clipboard.writeText(currentCliCommand);
    statusMessage = "📋 Comando CLI copiado al portapapeles.";
  }
</script>

<main class="h-screen w-screen flex flex-col overflow-hidden select-none bg-slate-100 dark:bg-brand-bg text-slate-800 dark:text-slate-100 font-sans transition-colors duration-200 {clipboardFlash ? 'ring-4 ring-primary-400 ring-inset' : ''}">
  
  <!-- CABECERA PRINCIPAL / NAVEGACIÓN -->
  <header class="h-14 border-b border-slate-200 dark:border-white/10 px-4 flex items-center justify-between bg-white/80 dark:bg-brand-surface/60 backdrop-blur-md z-20 shrink-0">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg overflow-hidden flex items-center justify-center p-0.5 bg-slate-100/80 dark:bg-white/5 border border-slate-200 dark:border-white/10 shadow-sm shrink-0">
        <img src="/icon.png" alt="LexiShield Logo" class="w-full h-full object-contain filter drop-shadow-[0_1px_2px_rgba(0,0,0,0.15)]" />
      </div>
      <div>
        <h1 class="text-sm font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
          LexiShield
          <span class="text-[10px] font-mono font-medium px-1.5 py-0.5 rounded bg-primary-500/10 text-primary-700 dark:text-primary-400 border border-primary-500/20">v{appVersion}</span>
        </h1>
      </div>
    </div>

    <!-- PESTAÑAS PRINCIPALES -->
    <nav class="flex items-center gap-1 bg-slate-200/90 dark:bg-slate-950/80 p-1 rounded-xl border border-slate-300 dark:border-white/10 text-xs font-semibold shadow-inner">
      {#if appConfig.gui.tabs.show_studio}
        <button
          on:click={() => (activeTab = "studio")}
          class="px-3 py-1.5 rounded-lg transition duration-150 flex items-center gap-1.5 {activeTab === 'studio' ? 'bg-white dark:bg-primary-500/20 text-slate-950 dark:text-primary-300 border border-slate-200 dark:border-primary-500/40 shadow-sm font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white border border-transparent'}"
        >
          <span>✨</span> Studio
        </button>
      {/if}

      {#if appConfig.gui.tabs.show_pairs}
        <button
          on:click={() => { activeTab = "pairs"; loadPairsForVault(vaultPath); }}
          class="px-3 py-1.5 rounded-lg transition duration-150 flex items-center gap-1.5 {activeTab === 'pairs' ? 'bg-white dark:bg-primary-500/20 text-slate-950 dark:text-primary-300 border border-slate-200 dark:border-primary-500/40 shadow-sm font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white border border-transparent'}"
        >
          <span>📑</span> Pares ({vaultMappings.length})
        </button>
      {/if}

      {#if appConfig.gui.tabs.show_vaults}
        <button
          on:click={() => { activeTab = "vaults"; refreshVaultsList(); }}
          class="px-3 py-1.5 rounded-lg transition duration-150 flex items-center gap-1.5 {activeTab === 'vaults' ? 'bg-white dark:bg-primary-500/20 text-slate-950 dark:text-primary-300 border border-slate-200 dark:border-primary-500/40 shadow-sm font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white border border-transparent'}"
        >
          <span>🗄️</span> Bóvedas Lexi
        </button>
      {/if}

      {#if appConfig.gui.tabs.show_watch}
        <button
          on:click={() => (activeTab = "watch")}
          class="px-3 py-1.5 rounded-lg transition duration-150 flex items-center gap-1.5 {activeTab === 'watch' ? 'bg-white dark:bg-primary-500/20 text-slate-950 dark:text-primary-300 border border-slate-200 dark:border-primary-500/40 shadow-sm font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white border border-transparent'}"
        >
          <span>📋</span> Portapapeles
        </button>
      {/if}
    </nav>

    <!-- ACCIONES DE CABECERA -->
    <div class="flex items-center gap-2">
      <!-- Indicador de Bóveda Activa y Desbloqueo -->
      <button
        on:click={() => {
          if (isVaultEncrypted && !isVaultUnlocked) {
            unlockTargetVault = vaultPath;
            showUnlockModal = true;
          }
        }}
        class="text-xs px-2.5 py-1 rounded-lg border font-mono flex items-center gap-1.5 transition {isVaultEncrypted ? (isVaultUnlocked ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-600 dark:text-emerald-400' : 'bg-amber-500/10 border-amber-500/30 text-amber-600 dark:text-amber-400 animate-pulse') : 'bg-slate-200 dark:bg-white/5 border-slate-300 dark:border-white/10 text-slate-700 dark:text-slate-300'}"
        title={vaultPath || "Bóveda por defecto"}
      >
        <span>{isVaultEncrypted ? (isVaultUnlocked ? "🔓 Desbloqueada" : "🔒 Bloqueada") : "📄 Bóveda Plana"}</span>
      </button>

      <!-- Botón Configuración -->
      <button
        on:click={() => (activeTab = "config")}
        class="w-8 h-8 rounded-lg flex items-center justify-center transition border {activeTab === 'config' ? 'bg-primary-500/20 border-primary-500/40 text-primary-600 dark:text-primary-400' : 'bg-slate-200 dark:bg-white/5 border-slate-300 dark:border-white/10 text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white'}"
        title="Configuración general"
      >
        ⚙️
      </button>

      <!-- Botón Modo DEBUG y Logs -->
      <button
        on:click={openLogsModal}
        class="w-8 h-8 rounded-lg flex items-center justify-center transition border {debugMode ? 'bg-amber-500/20 border-amber-500/40 text-amber-600 dark:text-amber-400 animate-pulse' : 'bg-slate-200 dark:bg-white/5 border-slate-300 dark:border-white/10 text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white'}"
        title="Registro de depuración (Logs)"
      >
        🐞
      </button>

      <!-- Botón Modo Privacidad / Desenfoque de Pantalla -->
      <button
        on:click={() => {
          appConfig.gui.privacy_blur = !appConfig.gui.privacy_blur;
          saveSettings();
        }}
        class="w-8 h-8 rounded-lg flex items-center justify-center transition border {privacyBlur ? 'bg-indigo-500/20 border-indigo-500/40 text-indigo-600 dark:text-indigo-400 font-bold' : 'bg-slate-200 dark:bg-white/5 border-slate-300 dark:border-white/10 text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white'}"
        title={privacyBlur ? "Modo Privacidad ACTIVO: Valores originales desenfocados (Pasa el cursor por encima para verlos)" : "Activar Modo Privacidad (Desenfocar datos originales para compartir pantalla)"}
      >
        {privacyBlur ? "🙈" : "👁️"}
      </button>

      <!-- Botón Tema Claro/Oscuro -->
      <button
        on:click={toggleTheme}
        class="w-8 h-8 rounded-lg bg-slate-200 dark:bg-white/5 hover:bg-slate-300 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-white/10 flex items-center justify-center transition"
        title="Alternar Tema Claro / Oscuro"
      >
        {#if darkMode}
          <span>☀️</span>
        {:else}
          <span>🌙</span>
        {/if}
      </button>
    </div>
  </header>

  <!-- CONTENIDO DINÁMICO POR PESTAÑA -->
  <div class="flex-1 overflow-auto p-4 flex flex-col min-h-0">
    {#if activeTab === "studio"}
      <!-- VISTA STUDIO UNIFICADA (TRANSFORMACIÓN + AUDITORÍA) -->
      <div in:fade={{ duration: 150 }} class="flex-1 flex flex-col gap-4 min-h-0">
        <!-- Barra Superior Minimalista de Studio con Selector de Sub-vistas y Controles Rápidos -->
        <div class="bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 px-4 py-2 flex items-center justify-between gap-3 shadow-sm text-xs flex-wrap">
          <!-- Grupo Izquierdo: Sub-vistas de Studio exclusivamente -->
          <div class="flex items-center gap-1.5 p-0.5 bg-slate-100 dark:bg-slate-900/60 rounded-xl border border-slate-200 dark:border-white/5">
            <button
              type="button"
              on:click={() => (studioSubView = "transform")}
              class="px-3 py-1 rounded-lg font-semibold transition flex items-center gap-1.5 {studioSubView === 'transform' ? 'bg-white dark:bg-primary-500/20 text-primary-600 dark:text-primary-300 shadow-sm border border-slate-200/60 dark:border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
            >
              <span>✨</span> Transformación
            </button>
            <button
              type="button"
              on:click={() => (studioSubView = "audit")}
              class="px-3 py-1 rounded-lg font-semibold transition flex items-center gap-1.5 {studioSubView === 'audit' ? 'bg-white dark:bg-primary-500/20 text-primary-600 dark:text-primary-300 shadow-sm border border-slate-200/60 dark:border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
            >
              <span>🔍</span> Desglose y Auditoría
              {#if visibleDetectedItems.length > 0}
                <span class="px-1.5 py-0.2 rounded-full text-[10px] bg-primary-500 text-slate-950 font-bold">
                  {visibleDetectedItems.length}
                </span>
              {/if}
            </button>
          </div>

          <!-- Grupo Derecho: Agrupación Semántica por Bloques Visuales (Opción 1) -->
          <div class="flex items-center gap-2.5 flex-wrap">
            <!-- Bloque 1: Ajustes de Procesamiento (Perfil y Auto-copiado) -->
            <div class="flex items-center gap-1.5">
              <!-- Selector de Perfil de Sensibilidad -->
              <select
                bind:value={activeSensitivityProfile}
                class="bg-slate-100 dark:bg-slate-900/60 border border-slate-200 dark:border-white/10 rounded-xl px-2.5 py-1 text-xs text-slate-700 dark:text-slate-300 font-medium focus:outline-none cursor-pointer"
                title="Filtrar detectores por perfil de sensibilidad"
              >
                <option value="all">🛡️ Perfil: Completo (Todos)</option>
                <option value="devops">🌐 Perfil: DevOps & Red</option>
                <option value="gdpr">🪪 Perfil: RGPD & Identidad</option>
              </select>

              <!-- Marcador Interactivo de Auto-copiado con indicador de estado (LED) -->
              <button
                type="button"
                on:click={() => {
                  autoCopyToClipboard = !autoCopyToClipboard;
                  statusMessage = autoCopyToClipboard
                    ? "📋 Auto-copiado al portapapeles ACTIVADO."
                    : "📋 Auto-copiado al portapapeles DESACTIVADO.";
                }}
                class="px-2.5 py-1 rounded-xl text-xs font-semibold transition flex items-center gap-1.5 border {autoCopyToClipboard ? 'bg-primary-500/15 border-primary-500/40 text-primary-700 dark:text-primary-300 shadow-xs' : 'bg-slate-100 dark:bg-slate-900/60 border-slate-200 dark:border-white/10 text-slate-400 dark:text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'}"
                title="Copiar automáticamente al portapapeles al proteger o restaurar (Clic para alternar)"
              >
                <span>📋</span>
                <span class="hidden sm:inline">Auto-copiar</span>
                <span
                  class="inline-block w-2 h-2 rounded-full transition-all duration-200 {autoCopyToClipboard ? 'bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.7)]' : 'bg-slate-300 dark:bg-slate-600'}"
                  aria-hidden="true"
                ></span>
              </button>
            </div>

            <!-- Divisor Vertical Sutil -->
            <span class="w-px h-4 bg-slate-200 dark:border-white/10 dark:bg-white/10 hidden sm:inline-block" aria-hidden="true"></span>

            <!-- Bloque 2: Acciones y Recuperación de Sesión -->
            <div class="flex items-center gap-1.5">
              {#if studioSubView === "transform"}
                <button
                  type="button"
                  on:click={scanCurrentStudioText}
                  disabled={loading || (!inputText.trim() && !outputText.trim())}
                  class="px-2.5 py-1 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-xl border border-slate-200 dark:border-white/10 font-semibold transition flex items-center gap-1 shadow-sm disabled:opacity-40"
                  title="Auditar tokens confidenciales del texto de los paneles sin modificarlo (Ctrl+Shift+A)"
                >
                  <span>🔍</span>
                  <span class="hidden md:inline">Auditar Texto</span>
                  <span class="md:hidden">Auditar</span>
                </button>
              {/if}

              <!-- Botón Historial de Sesión -->
              <button
                type="button"
                on:click={() => (showSessionHistoryModal = true)}
                class="px-2.5 py-1 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-xl border border-slate-200 dark:border-white/10 font-semibold transition flex items-center gap-1.5 shadow-sm"
                title="Ver historial temporal de sesiones en memoria RAM"
              >
                <span>⏱️</span>
                <span class="hidden sm:inline">Historial</span>
                {#if sessionHistory.length > 0}
                  <span class="px-1.5 py-0.2 rounded-full text-[10px] bg-primary-500 text-slate-950 font-bold">
                    {sessionHistory.length}
                  </span>
                {/if}
              </button>
            </div>

            <!-- Divisor Vertical Sutil -->
            <span class="w-px h-4 bg-slate-200 dark:border-white/10 dark:bg-white/10 hidden sm:inline-block" aria-hidden="true"></span>

            <!-- Bloque 3: Soporte y Atajos -->
            <div class="flex items-center gap-1.5">
              <!-- Botón Atajos de Teclado -->
              <button
                type="button"
                on:click={() => (showKeyboardShortcutsModal = true)}
                class="px-2.5 py-1 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 rounded-xl border border-slate-200 dark:border-white/10 font-semibold transition flex items-center gap-1 shadow-sm"
                title="Ver guía de atajos de teclado rápidos (o pulsa '?')"
              >
                <span>⌨️</span>
                <span class="hidden lg:inline text-[11px]">Atajos</span>
              </button>

              <button
                on:click={() => openHelp('studio')}
                class="px-2.5 py-1 bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-300 rounded-xl border border-primary-500/30 font-bold transition flex items-center gap-1 shadow-sm"
                title="Ver guía de uso de Studio y Auditoría"
              >
                <span>❔</span>
                <span class="hidden sm:inline">Ayuda</span>
              </button>
            </div>
          </div>
        </div>

        {#if studioSubView === "transform"}
          {#if !dismissedTips.studio}
            <div in:slide={{ duration: 150 }} class="p-3.5 bg-gradient-to-r from-primary-500/10 via-brand-surface/40 to-primary-500/5 rounded-2xl border border-primary-500/20 flex items-center justify-between gap-3 text-xs shadow-sm">
              <div class="flex items-center gap-3">
                <span class="text-lg">✨</span>
                <p class="text-slate-700 dark:text-slate-300">
                  <strong class="text-primary-600 dark:text-primary-400">¿Cómo usar Studio?</strong> Escribe o pega texto en <em>Texto claro</em> para protegerlo antes de enviarlo a la IA, o arrastra cualquier archivo o carpeta encima. Al recibir la respuesta de la IA, pégala en <em>Texto protegido</em> y pulsa <em>Restaurar datos</em> para recuperar los valores originales.
                </p>
              </div>
              <div class="flex items-center gap-2 shrink-0">
                <button on:click={() => openHelp('studio')} class="text-[11px] font-bold text-primary-600 dark:text-primary-400 hover:underline">Ver detalles</button>
                <button on:click={() => dismissTip('studio')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white px-1">✕</button>
              </div>
            </div>
          {/if}

          <!-- Paneles de Texto claro y Texto protegido con Puente Central -->
          <div class="flex-1 min-w-0 flex flex-col md:flex-row gap-3 min-h-0 items-stretch relative">
            <!-- Panel Izquierdo: Texto claro -->
            <div class="flex-1 min-w-0 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border {deobfuscateSuccessFlash ? 'border-emerald-500/80 ring-2 ring-emerald-500/40 shadow-emerald-500/20' : studioDirection === 'obfuscate' ? 'border-primary-500/60 shadow-md ring-1 ring-primary-500/20' : 'border-slate-200 dark:border-white/10'} p-4 flex flex-col min-h-0 frosted-card transition duration-150">
              <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-white/5 mb-2 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <span class="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
                    <span>📥</span> Texto claro
                  </span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20" title="Texto original en claro">
                    Texto en claro
                  </span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-medium bg-slate-100 dark:bg-white/5 text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-white/10 flex items-center gap-1" title="Admite arrastrar archivos y carpetas">
                    <span>📁</span> Arrastra archivo o carpeta
                  </span>
                </div>
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="text-[10px] font-mono text-slate-400">
                    {inputLinesCount} lín · {inputCharCount} car · {inputSizeKB}
                  </span>
                  <button
                    type="button"
                    on:click={() => (showTokenBadges = !showTokenBadges)}
                    class="flex items-center gap-1 px-2.5 py-0.5 rounded text-xs font-semibold transition {showTokenBadges ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-white/5'}"
                    title="Alternar vista de badges coloreados en tokens confidenciales"
                  >
                    <span>👁️</span> Badges
                  </button>
                  <button
                    on:click={copyInputWithFeedback}
                    disabled={!inputText}
                    class="flex items-center gap-1 px-2.5 py-0.5 rounded text-xs font-semibold transition {copiedInputFlash ? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 font-bold border border-emerald-500/30' : 'text-primary-600 dark:text-primary-400 hover:bg-primary-500/10'} disabled:opacity-40"
                    title="Copiar texto claro (Ctrl+Shift+C)"
                  >
                    {#if copiedInputFlash}
                      <span>✓</span> ¡Copiado!
                    {:else}
                      <span>📋</span> Copiar
                    {/if}
                  </button>
                </div>
              </div>
              <div class="flex-1 min-w-0 flex overflow-hidden rounded-xl border border-slate-200 dark:border-white/5 bg-slate-50 dark:bg-slate-950/60 focus-within:ring-1 focus-within:ring-primary-500">
                {#if showTokenBadges && inputText.trim()}
                  <div class="flex-1 w-full min-w-0 max-w-full p-3 {editorFontSizeClass} font-mono leading-5 overflow-y-auto overflow-x-hidden select-text whitespace-pre-wrap break-words {privacyBlur ? 'privacy-blurred' : ''}">
                    {@html renderHighlightedTokens(inputText, false)}
                  </div>
                {:else}
                  <pre
                    bind:this={inputGutterEl}
                    class="select-none py-3 px-2 text-right font-mono {editorFontSizeClass} leading-5 text-slate-400 dark:text-slate-600 bg-slate-100/70 dark:bg-slate-900/60 border-r border-slate-200 dark:border-white/5 overflow-hidden min-w-[2.8rem] m-0"
                    aria-hidden="true"
                  >{inputGutterText}</pre>
                  <textarea
                    bind:value={inputText}
                    on:input={() => { studioDirection = "obfuscate"; }}
                    on:focus={() => { if (inputText.trim()) studioDirection = "obfuscate"; }}
                    on:scroll={(e) => { if (inputGutterEl) inputGutterEl.scrollTop = e.currentTarget.scrollTop; }}
                    placeholder="Pega o escribe aquí el texto en claro, o arrastra un archivo o carpeta para PROTEGER..."
                    class="flex-1 w-full bg-transparent p-3 {editorFontSizeClass} font-mono leading-5 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none resize-none overflow-auto select-text whitespace-pre {privacyBlur ? 'privacy-blurred' : ''}"
                  ></textarea>
                {/if}
              </div>
            </div>

            <!-- Puente Central Direccional entre Paneles -->
            <div class="flex md:flex-col items-center justify-center gap-2 py-1 md:py-0 shrink-0">
              {#if studioDirection === "obfuscate"}
                <button
                  on:click={handleObfuscate}
                  disabled={loading || !inputText.trim()}
                  class="px-3.5 py-2.5 bg-gradient-to-r from-primary-600 to-primary-400 hover:from-primary-500 hover:to-primary-300 text-slate-950 font-bold rounded-xl shadow-md shadow-primary-500/20 transition duration-150 flex items-center justify-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed group text-xs"
                  title="Proteger datos (Ctrl+Enter)"
                >
                  {#if loading}
                    <span class="animate-spin">⏳</span>
                  {:else}
                    <span>🛡️</span>
                  {/if}
                  <span>Proteger</span>
                  <span class="font-extrabold group-hover:translate-x-0.5 transition">➔</span>
                </button>
              {:else}
                <button
                  on:click={handleDeobfuscate}
                  disabled={loading || !outputText.trim()}
                  class="px-3.5 py-2.5 bg-gradient-to-r from-primary-600 to-primary-400 hover:from-primary-500 hover:to-primary-300 text-slate-950 font-bold rounded-xl shadow-md shadow-primary-500/20 transition duration-150 flex items-center justify-center gap-1.5 disabled:opacity-40 disabled:cursor-not-allowed group text-xs"
                  title="Restaurar datos (Ctrl+Enter)"
                >
                  {#if loading}
                    <span class="animate-spin">⏳</span>
                  {:else}
                    <span class="font-extrabold group-hover:-translate-x-0.5 transition">⮜</span>
                  {/if}
                  <span>Restaurar</span>
                  <span>🔄</span>
                </button>
              {/if}

              <button
                on:click={toggleStudioDirection}
                class="p-2 rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300 text-xs border border-slate-200 dark:border-white/10 shadow-sm transition"
                title="Cambiar dirección (Ctrl+Shift+X)"
              >
                <span>⇄</span>
              </button>

              <button
                on:click={scanCurrentStudioText}
                disabled={loading || (!inputText.trim() && !outputText.trim())}
                class="p-2 rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300 text-xs border border-slate-200 dark:border-white/10 shadow-sm transition disabled:opacity-40"
                title="Auditar tokens sin modificar el texto (Ctrl+Shift+A)"
              >
                <span>🔍</span>
              </button>
            </div>

            <!-- Panel Derecho: Texto protegido -->
            <div class="flex-1 min-w-0 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border {studioDirection === 'deobfuscate' ? 'border-primary-500/60 shadow-md ring-1 ring-primary-500/20' : 'border-slate-200 dark:border-white/10'} p-4 flex flex-col min-h-0 frosted-card transition duration-150">
              <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-white/5 mb-2 gap-2 flex-wrap">
                <div class="flex items-center gap-2">
                  <span class="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider flex items-center gap-1.5">
                    <span>🛡️</span> Texto protegido
                  </span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-500/10 text-amber-700 dark:text-amber-400 border border-amber-500/20" title="Texto con datos protegidos por seudónimos">
                    Texto protegido
                  </span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-medium bg-slate-100 dark:bg-white/5 text-slate-600 dark:text-slate-300 border border-slate-200 dark:border-white/10 flex items-center gap-1" title="Admite arrastrar archivos para restaurar">
                    <span>📄</span> Arrastra archivo
                  </span>
                </div>
                <div class="flex items-center gap-2 flex-wrap">
                  <span class="text-[10px] font-mono text-slate-400">
                    {outputLinesCount} lín · {outputCharCount} car · {outputSizeKB}
                  </span>
                  <div class="flex items-center gap-0.5 bg-slate-100 dark:bg-slate-900/60 p-0.5 rounded-lg border border-slate-200 dark:border-white/5 text-[11px]">
                    <button
                      type="button"
                      on:click={() => (studioOutputViewMode = "editor")}
                      class="px-2 py-0.5 rounded font-semibold transition {studioOutputViewMode === 'editor' ? 'bg-white dark:bg-brand-surface text-primary-600 dark:text-primary-300 shadow-xs' : 'text-slate-500 hover:text-slate-800 dark:hover:text-slate-200'}"
                      title="Vista editor editable"
                    >
                      📄 Editor
                    </button>
                    <button
                      type="button"
                      on:click={() => (studioOutputViewMode = "markdown")}
                      class="px-2 py-0.5 rounded font-semibold transition {studioOutputViewMode === 'markdown' ? 'bg-white dark:bg-brand-surface text-primary-600 dark:text-primary-300 shadow-xs' : 'text-slate-500 hover:text-slate-800 dark:hover:text-slate-200'}"
                      title="Vista previa Markdown formateada para prompts de IA"
                    >
                      👁️ Markdown
                    </button>
                  </div>
                  {#if studioOutputViewMode === "editor"}
                    <button
                      type="button"
                      on:click={() => (showTokenBadges = !showTokenBadges)}
                      class="flex items-center gap-1 px-2.5 py-0.5 rounded text-xs font-semibold transition {showTokenBadges ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-white/5'}"
                      title="Alternar vista de badges coloreados en tokens seudonimizados"
                    >
                      <span>👁️</span> Badges
                    </button>
                  {/if}
                  <button
                    on:click={copyOutputWithFeedback}
                    disabled={!outputText}
                    class="flex items-center gap-1 px-2.5 py-0.5 rounded text-xs font-semibold transition {copiedOutputFlash ? 'bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 font-bold border border-emerald-500/30' : 'text-primary-600 dark:text-primary-400 hover:bg-primary-500/10'} disabled:opacity-40"
                    title="Copiar texto protegido (Ctrl+Shift+C)"
                  >
                    {#if copiedOutputFlash}
                      <span>✓</span> ¡Copiado!
                    {:else}
                      <span>📋</span> Copiar
                    {/if}
                  </button>
                </div>
              </div>
              <div class="flex-1 min-w-0 flex overflow-hidden rounded-xl border border-slate-200 dark:border-white/5 bg-slate-50 dark:bg-slate-950/60 focus-within:ring-1 focus-within:ring-primary-500">
                {#if studioOutputViewMode === "markdown"}
                  <div class="flex-1 w-full min-w-0 max-w-full p-4 overflow-y-auto overflow-x-hidden select-text prose dark:prose-invert max-w-none text-xs leading-relaxed font-sans whitespace-pre-wrap break-words">
                    {@html renderMarkdownPreview(outputText)}
                  </div>
                {:else if showTokenBadges && outputText.trim()}
                  <div class="flex-1 w-full min-w-0 max-w-full p-3 {editorFontSizeClass} font-mono leading-5 overflow-y-auto overflow-x-hidden select-text whitespace-pre-wrap break-words">
                    {@html renderHighlightedTokens(outputText, true)}
                  </div>
                {:else}
                  <pre
                    bind:this={outputGutterEl}
                    class="select-none py-3 px-2 text-right font-mono {editorFontSizeClass} leading-5 text-slate-400 dark:text-slate-600 bg-slate-100/70 dark:bg-slate-900/60 border-r border-slate-200 dark:border-white/5 overflow-hidden min-w-[2.8rem] m-0"
                    aria-hidden="true"
                  >{outputGutterText}</pre>
                  <textarea
                    bind:value={outputText}
                    on:input={() => { studioDirection = "deobfuscate"; }}
                    on:focus={() => { if (outputText.trim()) studioDirection = "deobfuscate"; }}
                    on:scroll={(e) => { if (outputGutterEl) outputGutterEl.scrollTop = e.currentTarget.scrollTop; }}
                    placeholder="Pega aquí la respuesta protegida de la IA, o arrastra un archivo para RESTAURAR..."
                    class="flex-1 w-full bg-transparent p-3 {editorFontSizeClass} font-mono leading-5 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none resize-none overflow-auto select-text whitespace-pre"
                  ></textarea>
                {/if}
              </div>
            </div>
          </div>

          <!-- Botonera de Acción Principal del Studio -->
          <div class="flex items-center justify-between gap-3 pt-1 flex-wrap">
            <div class="flex items-center gap-2">
              <button
                on:click={toggleStudioDirection}
                class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 text-xs font-medium rounded-xl border border-slate-200 dark:border-white/10 transition flex items-center gap-1.5 shadow-sm"
                title="Alternar sentido de procesamiento (Ctrl+Shift+X)"
              >
                <span>⇄</span>
                <span>Modo actual: <strong>{studioDirection === 'obfuscate' ? 'Proteger (Texto claro ➔ Texto protegido)' : 'Restaurar (Texto protegido ➔ Texto claro)'}</strong></span>
              </button>
              <button
                on:click={() => {
                  if (inputText || outputText) {
                    pushToSessionHistory("clear");
                    inputText = "";
                    outputText = "";
                    statusMessage = "Ambos paneles han sido limpiados (guardado en historial de sesión).";
                  }
                }}
                disabled={!inputText && !outputText}
                class="px-2.5 py-1.5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 text-xs transition disabled:opacity-40"
                title="Limpiar ambos paneles (Ctrl+Shift+L)"
              >
                Limpiar todo
              </button>
            </div>

            <div class="flex items-center gap-3">
              <button
                on:click={scanCurrentStudioText}
                disabled={loading || (!inputText.trim() && !outputText.trim())}
                class="px-4 py-2 bg-slate-100 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-300 text-xs font-bold rounded-xl border border-slate-200 dark:border-white/10 transition flex items-center gap-1.5 shadow-sm disabled:opacity-40"
                title="Auditar tokens confidenciales del texto actual sin modificarlo (Ctrl+Shift+A)"
              >
                <span>🔍</span>
                Auditar Tokens
              </button>
              {#if studioDirection === "obfuscate"}
                <button
                  on:click={handleObfuscate}
                  disabled={loading || !inputText.trim()}
                  class="px-6 py-2 bg-gradient-to-r from-primary-600 to-primary-400 hover:from-primary-500 hover:to-primary-300 text-slate-950 text-xs font-bold rounded-xl transition duration-150 shadow-lg shadow-primary-500/20 flex items-center gap-2 disabled:opacity-50"
                  title="Proteger datos y auto-copiar (Ctrl+Enter)"
                >
                  {#if loading}
                    <span class="animate-spin">⏳</span>
                  {:else}
                    <span>🛡️</span>
                  {/if}
                  Proteger datos ➔
                </button>
              {:else}
                <button
                  on:click={handleDeobfuscate}
                  disabled={loading || !outputText.trim()}
                  class="px-6 py-2 bg-gradient-to-r from-primary-600 to-primary-400 hover:from-primary-500 hover:to-primary-300 text-slate-950 text-xs font-bold rounded-xl transition duration-150 shadow-lg shadow-primary-500/20 flex items-center gap-2 disabled:opacity-50"
                  title="Restaurar datos y auto-copiar (Ctrl+Enter)"
                >
                  {#if loading}
                    <span class="animate-spin">⏳</span>
                  {:else}
                    <span>🔄</span>
                  {/if}
                  ⮜ Restaurar datos
                </button>
              {/if}
            </div>
          </div>

        {:else if studioSubView === "audit"}
          <!-- SUB-VISTA DESGLOSE Y AUDITORÍA DE TOKENS -->
          <div in:fade={{ duration: 150 }} class="flex-1 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-5 flex flex-col min-h-0 shadow-lg">
            <div class="flex flex-wrap items-center justify-between gap-4 mb-3 pb-2 border-b border-slate-200 dark:border-white/5">
              <div>
                <h2 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                  <span>🔍</span> Desglose y Auditoría de Tokens Confidenciales
                </h2>
                <p class="text-xs text-slate-500 dark:text-slate-400">Inspecciona y clasifica tokens detectados sin modificarlos. Desmarca casillas si deseas omitir falsos positivos.</p>
              </div>
              
              <div class="flex items-center gap-3">
                <label class="flex items-center gap-1.5 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
                  <input type="checkbox" bind:checked={recursiveScan} class="rounded accent-primary-500" />
                  <span>Recursivo (subdirectorios)</span>
                </label>

                <button
                  on:click={() => (showTextScanPanel = !showTextScanPanel)}
                  class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition flex items-center gap-1.5 shadow-sm"
                  title="Pegar fragmento de texto para auditar tokens confidenciales"
                >
                  <span>📋</span> Fragmento Adicional
                </button>

                <button
                  on:click={() => (studioSubView = "transform")}
                  class="px-3 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-300 text-xs font-bold rounded-xl border border-primary-500/30 transition flex items-center gap-1.5 shadow-sm"
                  title="Volver a los editores de transformación"
                >
                  <span>✨</span> Volver a Editores
                </button>
              </div>
            </div>

            {#if showTextScanPanel}
              <div in:slide={{ duration: 150 }} class="mb-4 p-4 bg-slate-50 dark:bg-slate-900/60 border border-slate-200 dark:border-white/10 rounded-2xl flex flex-col gap-3 shadow-sm">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-bold text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                    <span>📋</span> Auditar Fragmento de Texto Copiado o Pegado
                  </span>
                  <button on:click={() => (showTextScanPanel = false)} class="text-xs text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
                    ✕ Cerrar
                  </button>
                </div>
                <textarea
                  bind:value={textToAudit}
                  placeholder="Pega aquí cualquier fragmento de texto, log o código para extraer y clasificar sus tokens confidenciales..."
                  class="w-full h-24 bg-white dark:bg-slate-950/80 rounded-xl border border-slate-200 dark:border-white/10 p-3 {editorFontSizeClass} font-mono text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-primary-500 resize-none overflow-auto select-text {privacyBlur ? 'privacy-blurred' : ''}"
                ></textarea>
                <div class="flex items-center justify-between flex-wrap gap-2">
                  <span class="text-[11px] text-slate-500 dark:text-slate-400">
                    Se identificarán tokens confidenciales y se clasificarán en la tabla inferior sin tocar el disco.
                  </span>
                  <div class="flex items-center gap-2">
                    {#if textToAudit.trim()}
                      <button
                        on:click={() => (textToAudit = "")}
                        class="px-3 py-1.5 bg-slate-200 hover:bg-slate-300 dark:bg-white/10 dark:hover:bg-white/20 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl transition flex items-center gap-1"
                        title="Limpiar el fragmento introducido"
                      >
                        <span>🗑️</span> Limpiar
                      </button>
                      <button
                        on:click={sendPastedTextToStudio}
                        class="px-3 py-1.5 bg-slate-200 hover:bg-slate-300 dark:bg-white/10 dark:hover:bg-white/20 text-slate-800 dark:text-slate-200 text-xs font-semibold rounded-xl transition flex items-center gap-1"
                        title="Transferir este texto a los paneles de Studio para procesarlo"
                      >
                        <span>➡️</span> Cargar en Editores
                      </button>
                    {/if}
                    <button
                      on:click={scanPastedText}
                      disabled={loading || !textToAudit.trim()}
                      class="px-4 py-1.5 bg-primary-600 hover:bg-primary-700 text-white text-xs font-bold rounded-xl shadow-sm transition disabled:opacity-50 flex items-center gap-1.5"
                    >
                      <span>🔍</span> Analizar Fragmento
                    </button>
                  </div>
                </div>
              </div>
            {/if}

            {#if detectedItems.length === 0 && !isScanningDirectory && !showTextScanPanel}
              <div class="mb-4 p-8 border-2 border-dashed border-slate-300 dark:border-white/15 rounded-2xl flex flex-col items-center justify-center text-center gap-3 bg-slate-50/50 dark:bg-slate-900/30 transition hover:border-primary-500/50">
                <span class="text-4xl">📂</span>
                <div class="space-y-1">
                  <p class="text-xs font-bold text-slate-800 dark:text-slate-200">
                    Arrastra y suelta aquí cualquier archivo o carpeta para auditar
                  </p>
                  <p class="text-[11px] text-slate-500 dark:text-slate-400 max-w-md mx-auto">
                    LexiShield analizará los tokens sensibles sin modificar el contenido. O puedes auditar directamente el texto cargado en los editores de Studio.
                  </p>
                </div>
                <div class="pt-1 flex items-center gap-2 flex-wrap justify-center">
                  {#if inputText.trim() || outputText.trim()}
                    <button
                      on:click={scanCurrentStudioText}
                      disabled={loading}
                      class="px-4 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 rounded-xl font-bold text-xs transition flex items-center gap-1.5 shadow-md shadow-primary-500/20"
                    >
                      <span>🔍</span> Auditar Texto de los Editores ({inputLinesCount} lín)
                    </button>
                  {/if}
                  <button
                    on:click={() => (showTextScanPanel = true)}
                    class="px-3.5 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 rounded-xl border border-slate-300 dark:border-white/10 text-xs font-semibold transition flex items-center gap-1.5 shadow-sm"
                  >
                    <span>📋</span> Analizar fragmento adicional
                  </button>
                </div>
              </div>
            {/if}

            {#if isScanningDirectory}
              <div in:fade={{ duration: 150 }} class="mb-4 p-4 bg-primary-500/10 border border-primary-500/30 rounded-xl flex flex-wrap items-center justify-between gap-4 text-xs">
                <div class="flex items-center gap-3 min-w-0 flex-1">
                  <div class="w-8 h-8 rounded-lg bg-primary-500/20 text-primary-500 flex items-center justify-center animate-spin text-base">
                    ⏳
                  </div>
                  <div class="min-w-0 flex-1">
                    <div class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                      <span>Escaneando carpeta en tiempo real...</span>
                      <span class="inline-block w-2 h-2 rounded-full bg-primary-500 animate-ping"></span>
                    </div>
                    <div class="text-[11px] font-mono text-slate-700 dark:text-slate-300 truncate font-semibold" title={scanCurrentFile}>
                      📄 {scanCurrentFile}
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-4 text-xs font-mono">
                  <span class="text-slate-700 dark:text-slate-300">Archivos: <strong class="text-primary-600 dark:text-primary-400 font-bold">{scanFilesCount}</strong></span>
                  <span class="text-slate-700 dark:text-slate-300">Entidades: <strong class="text-amber-600 dark:text-amber-400 font-bold">{scanEntitiesCount}</strong></span>
                  
                  <button
                    on:click={cancelScan}
                    class="px-3 py-1.5 bg-rose-500/10 hover:bg-rose-500/20 text-rose-600 dark:text-rose-400 font-bold rounded-lg border border-rose-500/20 transition flex items-center gap-1"
                  >
                    <span>🛑</span> Cancelar
                  </button>
                </div>
              </div>
            {:else if auditSummary}
              <div class="mb-4 p-3 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 flex flex-wrap items-center justify-between gap-3 text-xs">
                <div class="flex items-center gap-2">
                  <span class="text-slate-500">Origen escaneado:</span>
                  <span class="font-mono text-slate-800 dark:text-slate-200 font-semibold truncate max-w-sm" title={auditSummary.path_analyzed}>{auditSummary.path_analyzed}</span>
                </div>
                <div class="flex items-center gap-4">
                  <span>Archivos analizados: <strong class="text-primary-700 dark:text-primary-400">{auditSummary.files_scanned}</strong></span>
                  <span>Archivos omitidos: <strong class="text-slate-500">{auditSummary.files_ignored}</strong></span>
                  <span>Entidades detectadas: <strong class="text-amber-600 dark:text-amber-400">{auditSummary.total_entities}</strong></span>
                </div>
              </div>
            {/if}

            {#if detectedItems.length > 0}
              <!-- Barra de Distribución Categórica de Entidades (Estilo GitHub Language Bar) -->
              {#if categoryStats}
                <div class="mb-3 p-3 bg-white dark:bg-brand-surface/80 rounded-xl border border-slate-200 dark:border-white/5 shadow-sm frosted-card">
                  <div class="flex items-center justify-between text-xs mb-2 font-medium flex-wrap gap-2">
                    <span class="text-slate-600 dark:text-slate-400 font-semibold">Distribución de Entidades Detectadas ({categoryStats.total}):</span>
                    <div class="flex items-center gap-3 text-[11px] font-mono flex-wrap">
                      {#if categoryStats.red.count > 0}
                        <span class="flex items-center gap-1 text-blue-600 dark:text-blue-400 font-medium">
                          <span class="w-2 h-2 rounded-full bg-blue-500"></span> 🌐 Red ({categoryStats.red.count} · {categoryStats.red.pct}%)
                        </span>
                      {/if}
                      {#if categoryStats.identidad.count > 0}
                        <span class="flex items-center gap-1 text-purple-600 dark:text-purple-400 font-medium">
                          <span class="w-2 h-2 rounded-full bg-purple-500"></span> 🪪 Identidad ({categoryStats.identidad.count} · {categoryStats.identidad.pct}%)
                        </span>
                      {/if}
                      {#if categoryStats.sistemas.count > 0}
                        <span class="flex items-center gap-1 text-emerald-600 dark:text-emerald-400 font-medium">
                          <span class="w-2 h-2 rounded-full bg-emerald-500"></span> 🖥️ Sistemas ({categoryStats.sistemas.count} · {categoryStats.sistemas.pct}%)
                        </span>
                      {/if}
                      {#if categoryStats.claves.count > 0}
                        <span class="flex items-center gap-1 text-amber-600 dark:text-amber-400 font-medium">
                          <span class="w-2 h-2 rounded-full bg-amber-500"></span> 💳 Contacto/Claves ({categoryStats.claves.count} · {categoryStats.claves.pct}%)
                        </span>
                      {/if}
                    </div>
                  </div>
                  <div class="h-2 w-full bg-slate-200 dark:bg-white/10 rounded-full overflow-hidden flex">
                    {#if categoryStats.red.count > 0}
                      <div style="width: {categoryStats.red.pct}%" class="bg-blue-500 transition-all duration-300" title="Red: {categoryStats.red.count} ({categoryStats.red.pct}%)"></div>
                    {/if}
                    {#if categoryStats.identidad.count > 0}
                      <div style="width: {categoryStats.identidad.pct}%" class="bg-purple-500 transition-all duration-300" title="Identidad: {categoryStats.identidad.count} ({categoryStats.identidad.pct}%)"></div>
                    {/if}
                    {#if categoryStats.sistemas.count > 0}
                      <div style="width: {categoryStats.sistemas.pct}%" class="bg-emerald-500 transition-all duration-300" title="Sistemas: {categoryStats.sistemas.count} ({categoryStats.sistemas.pct}%)"></div>
                    {/if}
                    {#if categoryStats.claves.count > 0}
                      <div style="width: {categoryStats.claves.pct}%" class="bg-amber-500 transition-all duration-300" title="Contacto/Claves: {categoryStats.claves.count} ({categoryStats.claves.pct}%)"></div>
                    {/if}
                  </div>
                </div>
              {/if}

              <!-- Barra de Acciones del Desglose/Escáner -->
              <div class="mb-3 flex flex-wrap items-center justify-between gap-2 p-2 bg-slate-100 dark:bg-slate-900/60 rounded-xl border border-slate-200 dark:border-white/5 text-xs">
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    on:click={() => handleBatchOmitScanItems(true)}
                    class="px-2.5 py-1.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-700 dark:text-amber-400 font-semibold rounded-lg border border-amber-500/20 transition flex items-center gap-1"
                    title="Marcar como omitidos (no se ofuscarán)"
                  >
                    <span>🚫</span> Omitir {selectedScanOriginals.length > 0 ? `(${selectedScanOriginals.length})` : 'Todos'}
                  </button>
                  <button
                    type="button"
                    on:click={() => handleBatchOmitScanItems(false)}
                    class="px-2.5 py-1.5 bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-700 dark:text-emerald-400 font-semibold rounded-lg border border-emerald-500/20 transition flex items-center gap-1"
                    title="Reactivar ofuscación"
                  >
                    <span>🛡️</span> Ofuscar {selectedScanOriginals.length > 0 ? `(${selectedScanOriginals.length})` : 'Todos'}
                  </button>
                </div>

                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    on:click={clearScanResults}
                    class="px-2.5 py-1.5 bg-slate-200 hover:bg-slate-300 dark:bg-white/10 dark:hover:bg-white/20 text-slate-700 dark:text-slate-300 font-semibold rounded-lg transition flex items-center gap-1"
                    title="Limpiar los resultados y reiniciar la auditoría"
                  >
                    <span>🗑️</span> Limpiar Resultados
                  </button>
                  <button
                    type="button"
                    on:click={handleSaveScanToVault}
                    class="px-3.5 py-1.5 bg-primary-600 hover:bg-primary-500 text-slate-950 font-bold rounded-lg transition shadow-md shadow-primary-500/20 flex items-center gap-1.5"
                    title="Guardar todos los mapeos en la bóveda activa aplicando las omisiones seleccionadas"
                  >
                    <span>💾</span> Guardar en Bóveda ({detectedItems.length})
                  </button>
                </div>
              </div>
            {/if}

            <div class="flex-1 overflow-auto rounded-xl border border-slate-200 dark:border-white/10 bg-slate-50 dark:bg-slate-950/80">
              <table class="w-full text-left font-mono {editorFontSizeClass}">
                <thead class="bg-slate-200 dark:bg-slate-900/90 text-slate-700 dark:text-slate-400 sticky top-0 border-b border-slate-300 dark:border-white/10">
                  <tr>
                    <th class="p-3 w-12 text-center">
                      <input
                        type="checkbox"
                        checked={visibleDetectedItems.length > 0 && selectedScanOriginals.length === visibleDetectedItems.length}
                        on:change={toggleSelectAllScanItems}
                        class="rounded accent-primary-500 cursor-pointer"
                        title="Seleccionar o desmarcar todos"
                      />
                    </th>
                    <th class="p-3 w-32">Estado</th>
                    <th class="p-3 w-40">Tipo de Entidad</th>
                    <th class="p-3">Valor Original Confidencial</th>
                    <th class="p-3">Seudónimo / Resultado</th>
                    <th class="p-3 w-28 text-center">Acción</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-200 dark:divide-white/5 text-slate-700 dark:text-slate-300">
                  {#each visibleDetectedItems as item (item.original)}
                    {@const cat = getDetectorCategory(item.detector_type)}
                    <tr class="hover:bg-black/5 dark:hover:bg-white/5 transition {item.omitted ? 'opacity-50' : ''}">
                      <td class="p-3 text-center">
                        <input
                          type="checkbox"
                          value={item.original}
                          bind:group={selectedScanOriginals}
                          class="rounded accent-primary-500 cursor-pointer"
                        />
                      </td>
                      <td class="p-3">
                        {#if item.omitted || item.pseudonym === '='}
                          <span class="px-2 py-0.5 rounded bg-amber-500/10 text-amber-600 dark:text-amber-400 text-[11px] font-semibold border border-amber-500/20">
                            🚫 Omitido
                          </span>
                        {:else}
                          <span class="px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 text-[11px] font-semibold border border-emerald-500/20">
                            🛡️ Ofuscado
                          </span>
                        {/if}
                      </td>
                      <td class="p-3">
                        <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded {cat.bgBadge} {cat.text} text-[11px] font-semibold border {cat.border}">
                          <span>{cat.icon}</span> {item.detector_type}
                        </span>
                      </td>
                      <td class="p-3 text-amber-600 dark:text-amber-300 font-semibold select-text {privacyBlur ? 'privacy-blurred' : ''}">{item.original}</td>
                      <td class="p-3 font-mono select-text {item.omitted ? 'text-slate-400 dark:text-slate-500' : 'text-primary-700 dark:text-primary-400'}">
                        {item.omitted || item.pseudonym === '=' ? '=' : item.pseudonym}
                      </td>
                      <td class="p-3 text-center">
                        <button
                          type="button"
                          on:click={() => toggleSingleScanOmit(item)}
                          class="px-2.5 py-1 rounded-lg text-[11px] font-semibold transition {item.omitted ? 'bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20' : 'bg-amber-500/10 hover:bg-amber-500/20 text-amber-600 dark:text-amber-400 border border-amber-500/20'}"
                        >
                          {item.omitted ? '✓ Ofuscar' : '🚫 Omitir'}
                        </button>
                      </td>
                    </tr>
                  {:else}
                    <tr>
                      <td colspan="6" class="p-12 text-center text-slate-400 dark:text-slate-500">
                        {#if isScanningDirectory || loading}
                          <div class="text-3xl mb-2 animate-spin inline-block">⏳</div>
                          <div class="font-medium text-primary-600 dark:text-primary-400">Escaneando archivos en curso...</div>
                          <div class="text-xs text-slate-600 dark:text-slate-300 mt-1 max-w-md mx-auto truncate font-mono font-semibold">📄 {scanCurrentFile}</div>
                        {:else}
                          <div class="text-3xl mb-2">🔍</div>
                          <div class="font-medium text-slate-600 dark:text-slate-300">No hay entidades auditadas todavía</div>
                          <div class="text-xs text-slate-400 mt-1">Arrastra archivos o carpetas, o pulsa "Auditar Texto" desde la vista de Transformación.</div>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === "pairs"}
      <!-- VISTA PARES DE TOKENS -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-6 flex flex-col min-h-0 shadow-lg">
        {#if !dismissedTips.pairs}
          <div transition:slide class="mb-4 p-3.5 bg-primary-500/10 border border-primary-500/30 rounded-xl flex items-start justify-between gap-3 text-xs text-slate-700 dark:text-slate-300">
            <div class="flex items-start gap-2.5">
              <span class="text-base">💡</span>
              <div>
                <p class="font-semibold text-slate-900 dark:text-white mb-0.5">Gestión de Pares de Tokens</p>
                <p>Cada par asocia un token confidencial (nombre, DNI, IBAN, correo o secreto) con su seudónimo generado. Puedes añadir pares manuales o editar los existentes. Toda modificación se sincroniza inmediatamente con el archivo cifrado de la bóveda.</p>
              </div>
            </div>
            <button on:click={() => dismissTip('pairs')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-xs px-2 py-0.5 rounded hover:bg-black/5 dark:hover:bg-white/5" title="No volver a mostrar">✕</button>
          </div>
        {/if}

        <div class="flex flex-wrap items-center justify-between gap-4 mb-4">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
              <span>📑</span> Pares de Tokens ({filteredPairs.length})
            </h2>
            <p class="text-xs text-slate-500 dark:text-slate-400">Gestiona, edita o añade correspondencias de tokens confidenciales y sus sustitutos en la bóveda activa.</p>
          </div>

          <div class="flex items-center gap-2">
            <button
              on:click={() => openHelp('pairs')}
              class="px-2.5 py-1.5 text-xs font-semibold rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-white/10 flex items-center gap-1 transition"
              title="Ver guía sobre gestión de tokens"
            >
              <span>❔</span> Ayuda
            </button>

            <!-- Selector de Bóveda -->
            <select
              bind:value={pairsSelectedVault}
              on:change={() => loadPairsForVault(pairsSelectedVault)}
              class="bg-slate-100 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-1.5 text-xs text-slate-800 dark:text-slate-200 font-mono"
            >
              {#each allVaultsList as v}
                <option value={v.path}>{v.name} ({v.size_formatted})</option>
              {/each}
            </select>

            <button
              on:click={() => (showAddPairModal = true)}
              class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-sm flex items-center gap-1"
            >
              <span>➕</span> Añadir Par
            </button>

            <button
              on:click={() => {
                const targetVault = pairsSelectedVault || vaultPath || 'default.lexi';
                const name = targetVault.split('/').pop();
                handleRestoreVaultBak(name);
              }}
              class="px-3 py-1.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-600 dark:text-amber-400 text-xs font-semibold rounded-xl border border-amber-500/30 transition flex items-center gap-1"
              title="Recuperar la última copia de seguridad automática (.bak) de esta bóveda"
            >
              <span>↺</span> Restaurar .bak
            </button>

            {#if selectedPairKeys.length > 0}
              <button
                on:click={() => handleBatchOmitPairs(true)}
                class="px-3 py-1.5 bg-amber-500/20 hover:bg-amber-500/30 text-amber-600 dark:text-amber-400 text-xs font-bold rounded-xl border border-amber-500/30 transition flex items-center gap-1"
                title="Configurar los elementos seleccionados como ignorados/omitidos"
              >
                <span>🚫</span> Omitir ({selectedPairKeys.length})
              </button>
              <button
                on:click={() => handleBatchOmitPairs(false)}
                class="px-3 py-1.5 bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-600 dark:text-emerald-400 text-xs font-bold rounded-xl border border-emerald-500/30 transition flex items-center gap-1"
                title="Reactivar ofuscación para los seleccionados"
              >
                <span>✓</span> Reactivar ({selectedPairKeys.length})
              </button>
              <button
                on:click={deleteSelectedPairs}
                class="px-3 py-1.5 bg-rose-500/20 hover:bg-rose-500/30 text-rose-600 dark:text-rose-400 text-xs font-bold rounded-xl border border-rose-500/30 transition flex items-center gap-1"
              >
                <span>🗑️</span> Borrar ({selectedPairKeys.length})
              </button>
            {/if}
          </div>
        </div>

        <!-- Filtros de Pares -->
        <div class="mb-4 flex items-center gap-3">
          <input
            type="text"
            bind:value={pairSearch}
            placeholder="Filtrar por original o seudónimo..."
            class="flex-1 bg-slate-50 dark:bg-slate-950/60 border border-slate-200 dark:border-white/5 rounded-xl px-3 py-1.5 text-xs text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-primary-500"
          />
          <button
            on:click={toggleSelectAllPairs}
            class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs rounded-xl border border-slate-300 dark:border-white/10 font-semibold"
          >
            {selectedPairKeys.length === filteredPairs.length && filteredPairs.length > 0 ? "Deseleccionar Todos" : "Seleccionar Todos"}
          </button>
        </div>

        <!-- Tabla de Pares -->
        <div class="flex-1 overflow-auto rounded-xl border border-slate-200 dark:border-white/10 bg-slate-50 dark:bg-slate-950/80">
          <table class="w-full text-left text-xs font-mono">
            <thead class="bg-slate-200 dark:bg-slate-900/90 text-slate-700 dark:text-slate-400 sticky top-0 border-b border-slate-300 dark:border-white/10">
              <tr>
                <th class="p-3 w-12 text-center">Sel</th>
                <th class="p-3 w-40">Tipo / Estado</th>
                <th class="p-3">Valor Original Confidencial</th>
                <th class="p-3">Seudónimo</th>
                <th class="p-3 w-36 text-center">Acciones</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 dark:divide-white/5 text-slate-700 dark:text-slate-300">
              {#each filteredPairs as item}
                <tr class="hover:bg-black/5 dark:hover:bg-white/5 transition {item.omitted || item.pseudonym === '=' ? 'opacity-80 bg-amber-500/5' : ''}">
                  <td class="p-3 text-center">
                    <input
                      type="checkbox"
                      checked={selectedPairKeys.includes(item.original)}
                      on:change={(e) => {
                        if (e.target.checked) {
                          selectedPairKeys = [...selectedPairKeys, item.original];
                        } else {
                          selectedPairKeys = selectedPairKeys.filter(k => k !== item.original);
                        }
                      }}
                      class="rounded accent-primary-500 cursor-pointer"
                    />
                  </td>
                  <td class="p-3">
                    {#if item.omitted || item.pseudonym === "="}
                      <span class="px-2 py-0.5 rounded bg-rose-500/10 text-rose-600 dark:text-rose-400 text-[11px] border border-rose-500/20 font-bold">
                        🚫 Ignorado
                      </span>
                    {:else}
                      <span class="px-2 py-0.5 rounded bg-primary-500/10 text-primary-700 dark:text-primary-400 text-[11px] border border-primary-500/20">
                        {item.detector_type}
                      </span>
                    {/if}
                  </td>
                  <td class="p-3 text-amber-600 dark:text-amber-300 font-semibold select-text {privacyBlur ? 'privacy-blurred' : ''}">{item.original}</td>
                  <td class="p-3 text-primary-700 dark:text-primary-400 select-text">
                    {#if editingPairOriginal === item.original}
                      <div class="flex items-center gap-1">
                        <input
                          type="text"
                          bind:value={editingPairPseudo}
                          class="bg-white dark:bg-slate-900 border border-primary-500 rounded px-2 py-0.5 text-xs text-primary-600 dark:text-primary-400 font-mono w-48"
                        />
                        <button on:click={() => saveInlineEditPair(item)} class="text-xs px-2 py-0.5 bg-primary-600 text-slate-950 font-bold rounded">✓</button>
                        <button on:click={() => (editingPairOriginal = null)} class="text-xs px-2 py-0.5 bg-slate-300 dark:bg-white/10 rounded">✕</button>
                      </div>
                    {:else if item.omitted || item.pseudonym === "="}
                      <span class="text-slate-400 italic">(Omitido: no se modifica)</span>
                    {:else}
                      <span>{item.pseudonym}</span>
                    {/if}
                  </td>
                  <td class="p-3 text-center">
                    <div class="flex items-center justify-center gap-2">
                      <button
                        on:click={() => handleToggleOmitPair(item)}
                        class="p-1 transition font-bold {item.omitted || item.pseudonym === '=' ? 'text-emerald-500 hover:text-emerald-700' : 'text-amber-500 hover:text-amber-700'}"
                        title={item.omitted || item.pseudonym === '=' ? "Reactivar ofuscación para esta entidad" : "Ignorar/Omitir esta entidad (mantener original)"}
                      >
                        {item.omitted || item.pseudonym === '=' ? '✓ Reactivar' : '🚫 Ignorar'}
                      </button>
                      <button
                        on:click={() => startInlineEditPair(item)}
                        class="p-1 text-slate-500 hover:text-slate-800 dark:hover:text-white transition"
                        title="Editar seudónimo"
                      >
                        ✏️
                      </button>
                      <button
                        on:click={() => deleteSinglePair(item.original)}
                        class="p-1 text-rose-500 hover:text-rose-700 transition"
                        title="Eliminar este par"
                      >
                        🗑️
                      </button>
                    </div>
                  </td>
                </tr>
              {:else}
                <tr>
                  <td colspan="5" class="p-12 text-center text-slate-400 dark:text-slate-500">
                    <div class="text-3xl mb-2">📑</div>
                    <div class="font-medium text-slate-600 dark:text-slate-300">No hay mapeos en esta bóveda</div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

    {:else if activeTab === "vaults"}
      <!-- VISTA GESTOR DE BÓVEDAS Y BACKUPS -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-6 flex flex-col min-h-0 shadow-lg">
        {#if !dismissedTips.vaults}
          <div transition:slide class="mb-4 p-3.5 bg-primary-500/10 border border-primary-500/30 rounded-xl flex items-start justify-between gap-3 text-xs text-slate-700 dark:text-slate-300">
            <div class="flex items-start gap-2.5">
              <span class="text-base">💡</span>
              <div>
                <p class="font-semibold text-slate-900 dark:text-white mb-0.5">Gestor de Bóvedas Lexi</p>
                <p>Las bóvedas (<code class="font-mono text-primary-600 dark:text-primary-400">.lexi</code>) son contenedores criptográficos aislados. Cada cliente, proyecto o entorno puede tener su propia bóveda independiente protegida con contraseña Argon2id. También puedes crear backups compactados en formato Bzip2.</p>
              </div>
            </div>
            <button on:click={() => dismissTip('vaults')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-xs px-2 py-0.5 rounded hover:bg-black/5 dark:hover:bg-white/5" title="No volver a mostrar">✕</button>
          </div>
        {/if}

        <div class="flex flex-wrap items-center justify-between gap-4 mb-4">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
              <span>🗄️</span> Bóvedas Lexi y Copias de Seguridad Bzip2
            </h2>
            <p class="text-xs text-slate-500 dark:text-slate-400">Administra todas las bóvedas en tu perfil (~/.lexishield) y realiza copias de seguridad de alta compresión.</p>
          </div>

          <div class="flex items-center gap-2">
            <button
              on:click={() => openHelp('vaults')}
              class="px-2.5 py-1.5 text-xs font-semibold rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-white/10 flex items-center gap-1 transition"
              title="Ver guía sobre bóvedas criptográficas"
            >
              <span>❔</span> Ayuda
            </button>
            <button
              on:click={() => openHelp('password_why')}
              class="px-2.5 py-1.5 text-xs font-semibold rounded-xl bg-amber-500/10 hover:bg-amber-500/20 text-amber-600 dark:text-amber-400 border border-amber-500/30 flex items-center gap-1 transition"
              title="¿Por qué es obligatoria la contraseña en las bóvedas?"
            >
              <span>🔐</span> ¿Por qué contraseña?
            </button>
            <button
              on:click={() => (showCreateVaultModal = true)}
              class="px-3 py-1.5 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-sm flex items-center gap-1"
            >
              <span>➕</span> Nueva Bóveda
            </button>
            <button
              on:click={handleImportVault}
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition flex items-center gap-1"
            >
              <span>📥</span> Importar Bóveda
            </button>
            <button
              on:click={() => (showBackupModal = true)}
              class="px-3 py-1.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-600 dark:text-amber-400 text-xs font-semibold rounded-xl border border-amber-500/30 transition flex items-center gap-1"
            >
              <span>📦</span> Copia de Seguridad Bzip2
            </button>
            <button
              on:click={handleRestoreBackup}
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition flex items-center gap-1"
            >
              <span>♻️</span> Restaurar Copia
            </button>
          </div>
        </div>

        <!-- Lista / Grid de Bóvedas -->
        <div class="flex-1 overflow-auto rounded-xl border border-slate-200 dark:border-white/10 bg-slate-50 dark:bg-slate-950/80">
          <table class="w-full text-left text-xs font-mono">
            <thead class="bg-slate-200 dark:bg-slate-900/90 text-slate-700 dark:text-slate-400 sticky top-0 border-b border-slate-300 dark:border-white/10">
              <tr>
                <th class="p-3">Nombre del Archivo</th>
                <th class="p-3 w-28">Tamaño</th>
                <th class="p-3 w-32">Estado</th>
                <th class="p-3">Ruta Absoluta</th>
                <th class="p-3 w-48 text-center">Acciones</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 dark:divide-white/5 text-slate-700 dark:text-slate-300">
              {#each allVaultsList as vault}
                <tr
                  on:dblclick={() => { if (vault.path !== vaultPath) checkAndLoadVault(vault.path, null); }}
                  class="hover:bg-black/5 dark:hover:bg-white/5 transition cursor-pointer {vault.path === vaultPath ? 'bg-primary-500/5 font-semibold' : ''}"
                  title={vault.path === vaultPath ? "Bóveda actualmente activa" : "Doble clic para activar"}
                >
                  <td class="p-3 text-slate-900 dark:text-white flex items-center gap-2">
                    <span>{vault.path === vaultPath ? '⭐' : '📁'}</span>
                    <span>{vault.name}</span>
                  </td>
                  <td class="p-3 text-slate-500">{vault.size_formatted}</td>
                  <td class="p-3">
                    <span class="px-2 py-0.5 rounded text-[11px] border {vault.is_encrypted ? 'bg-amber-500/10 border-amber-500/30 text-amber-600 dark:text-amber-400' : 'bg-slate-200 dark:bg-white/5 border-slate-300 dark:border-white/10 text-slate-600 dark:text-slate-400'}">
                      {vault.is_encrypted ? '🔒 Cifrada' : '🔓 Plana'}
                    </span>
                  </td>
                  <td class="p-3 text-slate-500 truncate max-w-xs" title={vault.path}>{vault.path}</td>
                  <td class="p-3 text-center">
                    <div class="flex items-center justify-center gap-2">
                      {#if vault.path === vaultPath}
                        <span class="px-2.5 py-0.5 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded border border-emerald-500/30 text-[11px] font-bold flex items-center gap-1 shadow-sm">
                          ⭐ Activa
                        </span>
                      {:else}
                        <button
                          on:click={() => checkAndLoadVault(vault.path, null)}
                          class="px-2 py-0.5 bg-primary-600/10 hover:bg-primary-600/20 text-primary-700 dark:text-primary-400 rounded border border-primary-500/20 text-[11px] font-bold transition"
                          title="Activar como bóveda actual"
                        >
                          Activar
                        </button>
                      {/if}
                      {#if vault.has_backup}
                        <button
                          on:click={() => handleRestoreVaultBak(vault.name)}
                          class="px-2 py-0.5 bg-amber-500/10 hover:bg-amber-500/20 text-amber-600 dark:text-amber-400 rounded border border-amber-500/30 text-[11px] font-bold"
                          title="Restaurar desde última copia de seguridad automática (.bak)"
                        >
                          ↺ .bak
                        </button>
                      {/if}
                      <button
                        on:click={() => openRenameVault(vault)}
                        class="p-1 text-slate-500 hover:text-slate-800 dark:hover:text-white"
                        title="Renombrar"
                      >
                        ✏️
                      </button>
                      <button
                        on:click={() => handleExportVault(vault)}
                        class="p-1 text-slate-500 hover:text-slate-800 dark:hover:text-white"
                        title="Exportar archivo"
                      >
                        📤
                      </button>
                      <button
                        on:click={() => handleDeleteVault(vault)}
                        class="p-1 text-rose-500 hover:text-rose-700"
                        title="Eliminar"
                      >
                        🗑️
                      </button>
                    </div>
                  </td>
                </tr>
              {:else}
                <tr>
                  <td colspan="5" class="p-12 text-center text-slate-400 dark:text-slate-500">
                    <div class="text-3xl mb-2">🗄️</div>
                    <div class="font-medium text-slate-600 dark:text-slate-300">No hay bóvedas disponibles</div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

    {:else if activeTab === "watch"}
      <!-- VISTA PORTAPAPELES (WATCH) -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-6 flex flex-col justify-between shadow-lg">
        <div>
          {#if !dismissedTips.watch}
            <div transition:slide class="mb-4 p-3.5 bg-primary-500/10 border border-primary-500/30 rounded-xl flex items-start justify-between gap-3 text-xs text-slate-700 dark:text-slate-300">
              <div class="flex items-start gap-2.5">
                <span class="text-base">💡</span>
                <div>
                  <p class="font-semibold text-slate-900 dark:text-white mb-0.5">Monitorización del Portapapeles (Watch Mode)</p>
                  <p>Al activar la escucha, cada vez que copies texto con Ctrl+C (o Cmd+C), LexiShield interceptará los tokens sensibles, los ofuscará en segundo plano con tu bóveda y colocará el texto seguro listo para pegar en IAs o chats. Si copias una respuesta de una IA, puedes poner la dirección en "Desofuscar" para recuperar los valores reales al instante.</p>
                </div>
              </div>
              <button on:click={() => dismissTip('watch')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-xs px-2 py-0.5 rounded hover:bg-black/5 dark:hover:bg-white/5" title="No volver a mostrar">✕</button>
            </div>
          {/if}

          <div class="flex items-center justify-between pb-4 border-b border-slate-200 dark:border-white/10 mb-6">
            <div>
              <h2 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>📋</span> Monitorización Activa del Portapapeles
              </h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Captura texto copiado al portapapeles y aplica transformaciones automáticas instantáneas.</p>
            </div>
            
            <div class="flex items-center gap-2">
              <button
                on:click={() => openHelp('watch')}
                class="px-2.5 py-1.5 text-xs font-semibold rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-white/10 flex items-center gap-1 transition"
                title="Ver guía de monitorización de portapapeles"
              >
                <span>❔</span> Ayuda
              </button>
              <button
                on:click={handleClearClipboard}
                class="px-3.5 py-1.5 bg-rose-500/10 hover:bg-rose-500/20 text-rose-600 dark:text-rose-400 rounded-xl border border-rose-500/30 text-xs font-bold transition flex items-center gap-1.5 shadow-sm"
              >
                <span>🧹</span> Vaciar Portapapeles
              </button>
            </div>
          </div>

          {#if clipboardError}
            <div class="mb-4 p-3.5 bg-amber-500/10 border border-amber-500/30 rounded-xl flex items-start gap-3 text-xs text-amber-800 dark:text-amber-200">
              <span class="text-base">⚠️</span>
              <div class="flex-1">
                <p class="font-bold text-slate-900 dark:text-white mb-0.5">Aviso de acceso al portapapeles de Linux</p>
                <p class="mb-1">{clipboardError}</p>
                <p class="text-[11px] text-slate-500 dark:text-slate-400">Si utilizas Wayland (KDE Plasma/GNOME), instala el paquete <code class="bg-black/10 dark:bg-white/10 px-1 py-0.5 rounded font-mono">wl-clipboard</code> (<code class="font-mono">sudo apt install wl-clipboard</code> o <code class="font-mono">sudo pacman -S wl-clipboard</code>) o asegúrate de que Klipper/XWayland esté activo.</p>
              </div>
            </div>
          {/if}

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-600 dark:text-slate-400">Dirección de Transformación</span>
              
              <div class="space-y-2 text-xs">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="radio" bind:group={watchDirection} value="obfuscate" class="accent-primary-500" />
                  <span>Ofuscar automáticamente (Original -> Seudónimo)</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="radio" bind:group={watchDirection} value="deobfuscate" class="accent-primary-500" />
                  <span>Desofuscar automáticamente (Seudónimo -> Original)</span>
                </label>
              </div>
            </div>

            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-3">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-600 dark:text-slate-400">Estado de Escucha</span>
              <div class="flex items-center gap-2">
                <span class="w-3 h-3 rounded-full {isWatching ? 'bg-emerald-500 animate-pulse' : 'bg-slate-400'}"></span>
                <span class="text-xs font-semibold text-slate-800 dark:text-slate-200">
                  {isWatching ? 'Escuchando portapapeles activamente...' : 'Detenido (inactivo)'}
                </span>
              </div>
              <p class="text-[11px] text-slate-400">Bóveda activa: <span class="font-mono text-primary-500">{vaultPath || 'default.lexi'}</span></p>
            </div>
          </div>

          <!-- Inspector de Tokens en Vivo -->
          <div class="mt-4 flex-1 flex flex-col min-h-0">
            <div class="flex items-center justify-between mb-2">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <span>🔍</span> Tokens y Entidades Interceptadas en Vivo
                {#if clipboardEvents.length > 0}
                  <span class="px-2 py-0.5 rounded-full bg-primary-500/10 text-primary-600 dark:text-primary-400 text-[10px] font-mono border border-primary-500/20">
                    {clipboardEvents.length} eventos
                  </span>
                {/if}
              </span>

              {#if clipboardEvents.length > 0}
                <button
                  on:click={() => (clipboardEvents = [])}
                  class="text-[11px] text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition"
                >
                  Limpiar Historial
                </button>
              {/if}
            </div>

            <div class="overflow-auto rounded-xl border border-slate-200 dark:border-white/10 bg-slate-50 dark:bg-slate-950/80 p-3 space-y-2.5 max-h-64 min-h-[160px]">
              {#each clipboardEvents as ev (ev.id)}
                <div class="p-3 bg-white dark:bg-slate-900/90 rounded-xl border border-slate-200 dark:border-white/5 shadow-sm space-y-2">
                  <div class="flex items-center justify-between text-xs">
                    <div class="flex items-center gap-2 font-mono">
                      <span class="text-slate-400 text-[11px]">{ev.timestamp}</span>
                      <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase {ev.direction === 'obfuscate' ? 'bg-primary-500/10 text-primary-600 dark:text-primary-400 border border-primary-500/20' : 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border border-indigo-500/20'}">
                        {ev.direction === 'obfuscate' ? 'Ofuscación' : 'Desofuscación'}
                      </span>
                    </div>
                    <span class="text-[11px] font-medium text-slate-500">{ev.tokens.length} entidades reconocidas</span>
                  </div>

                  <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs font-mono pt-1">
                    <div class="p-2 rounded bg-slate-100 dark:bg-slate-950/80 border border-slate-200 dark:border-white/5">
                      <span class="block text-[10px] text-slate-400 font-sans uppercase font-bold mb-1">Texto Capturado:</span>
                      <p class="truncate text-slate-700 dark:text-slate-300 select-all {privacyBlur ? 'privacy-blurred' : ''}" title={ev.originalText}>{ev.originalText}</p>
                    </div>
                    <div class="p-2 rounded bg-slate-100 dark:bg-slate-950/80 border border-slate-200 dark:border-white/5">
                      <span class="block text-[10px] text-primary-500 font-sans uppercase font-bold mb-1">Resultado en Portapapeles:</span>
                      <p class="truncate text-primary-700 dark:text-primary-300 select-all" title={ev.transformedText}>{ev.transformedText}</p>
                    </div>
                  </div>

                  {#if ev.tokens.length > 0}
                    <div class="flex flex-wrap gap-1.5 pt-1">
                      {#each ev.tokens as tok}
                        <div class="inline-flex items-center gap-1.5 px-2 py-1 bg-slate-100 dark:bg-slate-800/80 border border-slate-200 dark:border-white/10 rounded-lg text-xs font-mono">
                          <span class="text-[10px] px-1 py-0.2 bg-primary-500/20 text-primary-600 dark:text-primary-300 rounded font-semibold">{tok.detector_type || 'Token'}</span>
                          <span class="text-amber-600 dark:text-amber-400 font-semibold {privacyBlur ? 'privacy-blurred' : ''}">{tok.original}</span>
                          <span class="text-slate-400">→</span>
                          <span class="text-primary-600 dark:text-primary-400">{tok.pseudonym}</span>
                        </div>
                      {/each}
                    </div>
                  {:else}
                    <p class="text-[11px] text-slate-400 italic">0 tokens confidenciales detectados (texto íntegro mantenido).</p>
                  {/if}
                </div>
              {:else}
                <div class="h-full flex flex-col items-center justify-center py-8 text-slate-400 dark:text-slate-500 text-center">
                  <div class="text-2xl mb-1">📋</div>
                  <p class="text-xs font-medium">Esperando actividad en el portapapeles...</p>
                  <p class="text-[11px] text-slate-400 mt-0.5">Al copiar texto con IPs, emails, claves o tokens, verás aquí su reconocimiento en vivo.</p>
                </div>
              {/each}
            </div>
          </div>
        </div>

        <div class="flex justify-end pt-4 border-t border-slate-200 dark:border-white/10">
          <button
            on:click={toggleWatchMode}
            class="px-6 py-2.5 rounded-xl text-xs font-bold transition duration-150 flex items-center gap-2 shadow-lg {isWatching ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-rose-500/20' : 'bg-primary-600 hover:bg-primary-500 text-slate-950 shadow-primary-500/20'}"
          >
            <span>{isWatching ? '⏹️' : '▶️'}</span>
            {isWatching ? 'Detener Monitorización' : 'Iniciar Monitorización en Vivo'}
          </button>
        </div>
      </div>

    {:else if activeTab === "config"}
      <!-- VISTA CONFIGURACIÓN -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-6 flex flex-col justify-between shadow-lg overflow-auto">
        <div class="space-y-6">
          {#if !dismissedTips.config}
            <div transition:slide class="mb-4 p-3.5 bg-primary-500/10 border border-primary-500/30 rounded-xl flex items-start justify-between gap-3 text-xs text-slate-700 dark:text-slate-300">
              <div class="flex items-start gap-2.5">
                <span class="text-base">💡</span>
                <div>
                  <p class="font-semibold text-slate-900 dark:text-white mb-0.5">Preferencias del Sistema</p>
                  <p>Ajusta qué pestañas se muestran en la barra de navegación, qué vista se abre al arrancar LexiShield, y las opciones de borrado automático seguro del portapapeles en el archivo de configuración estándar <code class="font-mono text-primary-600 dark:text-primary-400">~/.lexishield/config.toml</code>.</p>
                </div>
              </div>
              <button on:click={() => dismissTip('config')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-xs px-2 py-0.5 rounded hover:bg-black/5 dark:hover:bg-white/5" title="No volver a mostrar">✕</button>
            </div>
          {/if}

          <div class="flex items-center justify-between pb-4 border-b border-slate-200 dark:border-white/10">
            <div>
              <h2 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>⚙️</span> Configuración General de LexiShield
              </h2>
              <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">Personaliza la ergonomía de la interfaz, comportamiento del portapapeles y persistencia.</p>
            </div>

            <button
              on:click={() => openHelp('config')}
              class="px-2.5 py-1.5 text-xs font-semibold rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-white/10 flex items-center gap-1 transition"
              title="Ver guía de configuración"
            >
              <span>❔</span> Ayuda
            </button>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Pestaña Inicial y Visibilidad -->
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">Pestañas y Arranque</span>
              
              <div>
                <label for="config-default-tab-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Pestaña Inicial Predeterminada:</label>
                <select
                  id="config-default-tab-select"
                  bind:value={appConfig.gui.default_tab}
                  class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white"
                >
                  <option value="studio">✨ Studio</option>
                  <option value="pairs">📑 Pares de Tokens</option>
                  <option value="vaults">🗄️ Bóvedas Lexi</option>
                  <option value="watch">📋 Portapapeles</option>
                </select>
              </div>

              <div class="space-y-2 text-xs">
                <span class="block font-medium text-slate-600 dark:text-slate-400">Pestañas Visibles en Barra Superior:</span>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" bind:checked={appConfig.gui.tabs.show_studio} class="rounded accent-primary-500" />
                  <span>Studio</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" bind:checked={appConfig.gui.tabs.show_pairs} class="rounded accent-primary-500" />
                  <span>Pares de Tokens</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" bind:checked={appConfig.gui.tabs.show_vaults} class="rounded accent-primary-500" />
                  <span>Bóvedas Lexi</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" bind:checked={appConfig.gui.tabs.show_watch} class="rounded accent-primary-500" />
                  <span>Portapapeles (Watch)</span>
                </label>
              </div>
            </div>

            <!-- Avisos y Auto-vaciado de Portapapeles -->
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">Portapapeles y Avisos</span>
              
              <div>
                <label for="config-clip-notify-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Aviso al capturar datos del portapapeles:</label>
                <select
                  id="config-clip-notify-select"
                  bind:value={appConfig.gui.clipboard.notification_type}
                  class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white"
                >
                  <option value="toast">🍞 Mensaje emergente (Toast en barra inferior)</option>
                  <option value="visual_flash">⚡ Parpadeo visual de ventana</option>
                  <option value="silent">🔕 Silencioso (Sin avisos)</option>
                </select>
              </div>

              <div class="space-y-2 text-xs">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" bind:checked={appConfig.gui.clipboard.auto_clear_enabled} class="rounded accent-primary-500" />
                  <span>Programar auto-vaciado del portapapeles tras procesar</span>
                </label>
                
                {#if appConfig.gui.clipboard.auto_clear_enabled}
                  <div class="flex items-center gap-2 pl-6">
                    <span class="text-slate-500">Vaciar tras:</span>
                    <input
                      type="number"
                      min="1"
                      max="60"
                      bind:value={appConfig.gui.clipboard.auto_clear_seconds}
                      class="w-16 bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded px-2 py-0.5 text-xs text-slate-900 dark:text-white text-center font-mono"
                    />
                    <span class="text-slate-500">segundos (defecto: 5s)</span>
                  </div>
                {/if}
              </div>

              <div class="pt-2 border-t border-slate-200 dark:border-white/5 text-xs">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" bind:checked={appConfig.gui.show_cli_dock} class="rounded accent-primary-500" />
                  <span>Mostrar barra inferior con comando CLI equivalente</span>
                </label>
              </div>
            </div>

            <!-- Selección de Archivos y Carpetas -->
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">Selección de Archivos y Carpetas</span>
              
              <div>
                <label for="config-file-selection-mode-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Método de ingesta preferido:</label>
                <select
                  id="config-file-selection-mode-select"
                  bind:value={appConfig.gui.file_selection_mode}
                  class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white"
                >
                  <option value="ambos">✨ Ambos (Arrastrar y Soltar + Botones)</option>
                  <option value="arrastrar">🖱️ Solo Arrastrar y Soltar (Drag & Drop)</option>
                  <option value="botones">🔘 Solo Botones de Selección</option>
                </select>
                <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-2">
                  Configura cómo interactuar con archivos en Studio y Escáner. Exige que al menos una opción permanezca activa en todo momento.
                </p>
              </div>
            </div>

            <!-- Escritorio y Salida de Archivos -->
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">Escritorio y Salida de Archivos</span>
              
              <div>
                <label for="config-output-suffix-input" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Sufijo para copias procesadas en Studio:</label>
                <input
                  id="config-output-suffix-input"
                  type="text"
                  bind:value={appConfig.gui.output_suffix}
                  placeholder="_obfuscated"
                  class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white font-mono"
                />
                <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-1">
                  Se añadirá automáticamente antes de la extensión (ej. <code>archivo{appConfig.gui.output_suffix || '_obfuscated'}.json</code>).
                </p>
              </div>

              <div class="pt-2 border-t border-slate-200 dark:border-white/5 space-y-2">
                <label for="config-close-action-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400">
                  Comportamiento al pulsar el botón de cerrar [X]:
                </label>
                <select
                  id="config-close-action-select"
                  bind:value={appConfig.gui.close_action}
                  on:change={() => {
                    appConfig.gui.close_to_tray = (appConfig.gui.close_action === "tray");
                    saveSettings();
                  }}
                  class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white font-medium"
                >
                  <option value="tray">🗄️ Minimizar a la bandeja del sistema (icono junto al reloj / segundo plano)</option>
                  <option value="taskbar">📌 Minimizar a la barra de tareas (como ventana minimizada)</option>
                  <option value="close">🚪 Cerrar la aplicación completamente (por defecto del sistema)</option>
                </select>
                <p class="text-[11px] text-slate-500 dark:text-slate-400">
                  {#if appConfig.gui.close_action === 'tray'}
                    Al pulsar [X], la ventana se oculta y LexiShield sigue protegiendo en segundo plano desde el área de notificación.
                  {:else if appConfig.gui.close_action === 'taskbar'}
                    Al pulsar [X], la ventana se minimiza a la barra de tareas inferior sin cerrarse.
                  {:else}
                    Al pulsar [X], la ventana y el proceso se cierran por completo.
                  {/if}
                </p>
              </div>
            </div>

            <!-- Aspecto, Tipografía y Privacidad -->
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">Aspecto y Accesibilidad</span>
              
              <div class="space-y-3">
                <div>
                  <label for="config-theme-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Tema Visual:</label>
                  <select
                    id="config-theme-select"
                    bind:value={appConfig.gui.theme}
                    on:change={applyThemeFromConfig}
                    class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white"
                  >
                    <option value="system">🖥️ Seguir tema del sistema</option>
                    <option value="dark">🌙 Modo Oscuro</option>
                    <option value="light">☀️ Modo Claro</option>
                  </select>
                </div>

                <div>
                  <span class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-2">Color de Acento de la Interfaz:</span>
                  <div class="grid grid-cols-2 sm:grid-cols-5 gap-2.5">
                    {#each [
                      { id: 'cyan', label: 'Cian Neón', color: 'bg-cyan-500', desc: 'Defecto' },
                      { id: 'emerald', label: 'Esmeralda', color: 'bg-emerald-500', desc: 'Ciber' },
                      { id: 'cobalt', label: 'Cobalto', color: 'bg-blue-600', desc: 'SOC' },
                      { id: 'purple', label: 'Amatista', color: 'bg-purple-600', desc: 'Auditoría' },
                      { id: 'amber', label: 'Ámbar', color: 'bg-amber-500', desc: 'Retro' }
                    ] as palette}
                      <button
                        type="button"
                        on:click={() => {
                          appConfig.gui.accent_color = palette.id;
                          applyAccentFromConfig();
                        }}
                        class="flex flex-col items-center gap-1 p-2 rounded-xl border text-center transition {appConfig.gui.accent_color === palette.id ? 'border-primary-500 bg-primary-500/10 ring-1 ring-primary-500' : 'border-slate-200 dark:border-white/10 hover:bg-black/5 dark:hover:bg-white/5'}"
                      >
                        <span class="w-6 h-6 rounded-full {palette.color} shadow-sm border-2 border-white dark:border-slate-900 flex items-center justify-center text-white text-[10px] font-bold">
                          {#if appConfig.gui.accent_color === palette.id}✓{/if}
                        </span>
                        <span class="text-xs font-semibold text-slate-800 dark:text-slate-200">{palette.label}</span>
                        <span class="text-[10px] text-slate-400">{palette.desc}</span>
                      </button>
                    {/each}
                  </div>
                </div>

                <div>
                  <label for="config-font-size-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Tamaño de letra en editores y tablas:</label>
                  <select
                    id="config-font-size-select"
                    bind:value={appConfig.gui.font_size}
                    class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white"
                  >
                    <option value="small">Compacta (11px) - Máxima densidad</option>
                    <option value="medium">Estándar (12px) - Equilibrada</option>
                    <option value="large">Ampliada (14px) - Pantallas 2K/4K</option>
                  </select>
                </div>

                <div class="pt-2 border-t border-slate-200 dark:border-white/5 text-xs">
                  <label class="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" bind:checked={appConfig.gui.privacy_blur} class="rounded accent-primary-500" />
                    <span>Activar Modo Privacidad (Desenfoque de pantalla) por defecto</span>
                  </label>
                </div>
              </div>
            </div>

            <!-- Seguridad y Cero Conocimiento -->
            <div class="p-4 bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 space-y-4">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">Seguridad y Bóvedas</span>
              
              <div>
                <label for="config-auto-lock-select" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1">Auto-bloqueo de bóveda por inactividad:</label>
                <select
                  id="config-auto-lock-select"
                  bind:value={appConfig.gui.auto_lock_minutes}
                  class="w-full bg-slate-100 dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1.5 text-xs text-slate-900 dark:text-white"
                >
                  <option value={0}>🛡️ Desactivado (Mantener desbloqueada)</option>
                  <option value={1}>⏱️ Bloquear tras 1 minuto sin actividad</option>
                  <option value={5}>⏱️ Bloquear tras 5 minutos sin actividad</option>
                  <option value={15}>⏱️ Bloquear tras 15 minutos sin actividad</option>
                  <option value={30}>⏱️ Bloquear tras 30 minutos sin actividad</option>
                </select>
                <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-2">
                  Al activarse, LexiShield descarga de la memoria RAM la contraseña y los mapeos si no se detecta actividad del usuario.
                </p>
              </div>
            </div>
          </div>
        </div>

        <div class="flex items-center justify-between pt-6 border-t border-slate-200 dark:border-white/10">
          <button
            on:click={async () => {
              if (confirm("¿Restablecer ~/.lexishield/config.toml a los valores por defecto?")) {
                await invokeTauri("reset_config_command");
                await loadSettings();
                statusMessage = "✅ Configuración restablecida a valores por defecto.";
              }
            }}
            class="px-4 py-2 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-600 dark:text-slate-400 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition"
          >
            Restablecer Valores por Defecto
          </button>

          <button
            on:click={saveSettings}
            class="px-6 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-lg shadow-primary-500/20"
          >
            Guardar Configuración
          </button>
        </div>
      </div>
    {/if}
  </div>

  <!-- BARRA INFERIOR DE COMANDO CLI EN VIVO (Conmutable) -->
  {#if appConfig.gui.show_cli_dock}
    <div class="h-9 bg-slate-900 dark:bg-black/90 border-t border-slate-800 dark:border-white/10 px-4 flex items-center justify-between text-[11px] font-mono shrink-0 z-10">
      <div class="flex items-center gap-2 overflow-hidden">
        <span class="text-primary-400 font-bold">$</span>
        <span class="text-slate-300 truncate select-text">{currentCliCommand}</span>
      </div>
      <button
        on:click={copyCurrentCliCommand}
        class="text-xs text-primary-400 hover:text-primary-300 font-semibold px-2 py-0.5 rounded bg-primary-500/10 border border-primary-500/20 transition shrink-0"
        title="Copiar comando de consola"
      >
        Copiar CLI
      </button>
    </div>
  {/if}

  <!-- BARRA DE ESTADO INFERIOR MODULAR TIPO IDE -->
  <footer class="h-7 bg-slate-200/90 dark:bg-slate-950 border-t border-slate-300 dark:border-white/10 px-3 flex items-center justify-between text-[11px] text-slate-600 dark:text-slate-400 shrink-0 select-none">
    <!-- Izquierda: Estado general / Mensaje activo -->
    <div class="flex items-center gap-2 truncate flex-1 mr-4">
      <span class="truncate select-text">{statusMessage}</span>
    </div>

    <!-- Derecha: Cápsulas modulares tipo VS Code / JetBrains -->
    <div class="flex items-center gap-1.5 shrink-0 font-mono text-[10px]">
      <!-- Bóveda activa -->
      <div class="flex items-center gap-1.5 px-2 py-0.5 rounded bg-slate-300/60 dark:bg-white/5 border border-slate-300/40 dark:border-white/5" title="Bóveda de seudónimos activa">
        <span class="w-1.5 h-1.5 rounded-full {isVaultUnlocked ? 'bg-emerald-500 animate-pulse' : 'bg-amber-500'}"></span>
        <span class="font-semibold text-slate-700 dark:text-slate-300">{vaultPath ? vaultPath.split('/').pop() : 'default.lexi'}</span>
      </div>

      <!-- Arquitectura Zero-Knowledge -->
      <div class="hidden sm:flex items-center gap-1 px-2 py-0.5 rounded bg-slate-300/60 dark:bg-white/5 border border-slate-300/40 dark:border-white/5" title="Cifrado local sin servidores externos">
        <span>🛡️</span> Zero-Knowledge
      </div>

      <!-- Formato de entrada detectado -->
      <div class="hidden md:flex items-center gap-1 px-2 py-0.5 rounded bg-primary-500/10 text-primary-700 dark:text-primary-300 border border-primary-500/20" title="Formato sintáctico detectado">
        <span>⚡</span> {detectedInputFormat}
      </div>

      <!-- Rendimiento / Telemetría del último reporte -->
      {#if lastReport}
        <div class="hidden lg:flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20" title="Rendimiento del último procesamiento">
          <span>⚡</span> {lastReport.elapsed_ms.toFixed(1)}ms · {lastReport.replacements_applied} repl.
        </div>
      {/if}

      <!-- Modo de Privacidad -->
      <div class="hidden sm:flex items-center gap-1 px-2 py-0.5 rounded bg-slate-300/60 dark:bg-white/5 border border-slate-300/40 dark:border-white/5" title="Modo de privacidad visual (desenfoque)">
        <span>{privacyBlur ? '🕶️ Protegido' : '👁️ Normal'}</span>
      </div>

      <!-- Codificación y Versión -->
      <div class="px-2 py-0.5 rounded bg-slate-300/60 dark:bg-white/5 border border-slate-300/40 dark:border-white/5 text-slate-500 dark:text-slate-400">
        UTF-8
      </div>
      <div class="px-2 py-0.5 rounded bg-primary-600/15 text-primary-700 dark:text-primary-400 font-semibold border border-primary-500/20">
        v2.0.1
      </div>
    </div>
  </footer>

  <!-- MODAL DE DESBLOQUEO INMEDIATO DE BÓVEDA -->
  {#if showUnlockModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-500 flex items-center justify-center text-xl">
            🔒
          </div>
          <div>
            <h3 class="text-sm font-bold text-slate-900 dark:text-white">Bóveda Cifrada Detectada</h3>
            <p class="text-xs text-slate-500 dark:text-slate-400">Se requiere contraseña para descifrar los mapeos.</p>
          </div>
        </div>

        <div class="p-3 bg-slate-50 dark:bg-slate-950 rounded-xl border border-slate-200 dark:border-white/5 text-xs font-mono text-slate-600 dark:text-slate-400 truncate">
          Ruta: {unlockTargetVault || vaultPath}
        </div>

        {#if unlockError}
          <div class="p-2.5 bg-rose-500/10 border border-rose-500/30 rounded-xl text-rose-600 dark:text-rose-400 text-xs font-medium">
            ❌ {unlockError}
          </div>
        {/if}

        <form on:submit|preventDefault={handleUnlockVaultSubmit} class="space-y-4">
          <div>
            <div class="flex items-center justify-between mb-1">
              <label for="unlock-pwd-input" class="block text-xs font-semibold text-slate-700 dark:text-slate-300">Contraseña de Desbloqueo:</label>
              <button
                type="button"
                on:click={() => openHelp('password_why')}
                class="text-[11px] text-primary-600 dark:text-primary-400 hover:underline flex items-center gap-1"
              >
                <span>🔐</span> ¿Por qué contraseña?
              </button>
            </div>
            <input
              id="unlock-pwd-input"
              use:autofocus
              type="password"
              bind:value={unlockPassword}
              placeholder="Introduce la contraseña..."
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-xs text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>

          <div class="flex items-center justify-end gap-2 pt-2">
            <button
              type="button"
              on:click={handleCancelUnlock}
              class="px-4 py-2 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition"
            >
              Cancelar
            </button>
            <button
              type="submit"
              class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-lg shadow-primary-500/20"
            >
              Desbloquear Bóveda
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- MODAL DE INICIALIZACIÓN DE BÓVEDA PRINCIPAL -->
  {#if showInitVaultModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-primary-500/10 border border-primary-500/20 text-primary-500 flex items-center justify-center text-xl">
            🔒
          </div>
          <div>
            <h3 class="text-sm font-bold text-slate-900 dark:text-white">Inicializar Bóveda Principal</h3>
            <p class="text-xs text-slate-500 dark:text-slate-400">Protege tu nueva bóveda con contraseña segura.</p>
          </div>
        </div>

        <div class="p-3 bg-slate-50 dark:bg-slate-950 rounded-xl border border-slate-200 dark:border-white/5 text-xs font-mono text-slate-600 dark:text-slate-400 truncate">
          Archivo: {initVaultTarget || vaultPath || 'default.lexi'}
        </div>

        <form on:submit|preventDefault={handleInitVaultSubmit} class="space-y-4">
          <div>
            <div class="flex items-center justify-between mb-1">
              <label for="init-vault-pwd" class="block text-xs font-semibold text-slate-700 dark:text-slate-300">Contraseña de Cifrado (Obligatoria):</label>
              <button
                type="button"
                on:click={() => openHelp('password_why')}
                class="text-[11px] text-primary-600 dark:text-primary-400 hover:underline flex items-center gap-1"
              >
                <span>🔐</span> ¿Por qué contraseña?
              </button>
            </div>
            <input
              id="init-vault-pwd"
              use:autofocus
              type="password"
              bind:value={initVaultPassword}
              placeholder="Introduce la contraseña para cifrar la bóveda..."
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-xs text-slate-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
            <p class="text-[11px] text-slate-400 mt-1">La bóveda se cifrará de forma inviolable con Argon2id + ChaCha20-Poly1305.</p>
          </div>

          <div class="flex items-center justify-end gap-2 pt-2">
            <button
              type="submit"
              class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-lg shadow-primary-500/20"
            >
              Inicializar y Cifrar Bóveda
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- MODAL AÑADIR PAR MANUAL -->
  {#if showAddPairModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
        <h3 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <span>➕</span> Añadir Par de Ofuscación
        </h3>
        
        <form on:submit|preventDefault={handleAddPairSubmit} class="space-y-3 text-xs">
          <div>
            <label for="add-pair-orig-input" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Valor Original Confidencial:</label>
            <input
              id="add-pair-orig-input"
              type="text"
              bind:value={newPairOriginal}
              placeholder="ej: Juan Pérez o 192.168.1.10"
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white font-mono focus:ring-1 focus:ring-primary-500 {privacyBlur ? 'privacy-blurred' : ''}"
            />
          </div>

          <div>
            <label for="add-pair-pseudo-input" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Seudónimo Ofuscado (o '=' para omitir):</label>
            <input
              id="add-pair-pseudo-input"
              type="text"
              bind:value={newPairPseudo}
              placeholder="ej: AnonUser_001 o 10.0.0.1"
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white font-mono focus:ring-1 focus:ring-primary-500"
            />
          </div>

          <div>
            <label for="add-pair-type-select" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Tipo de Detector:</label>
            <select
              id="add-pair-type-select"
              bind:value={newPairType}
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white font-mono"
            >
              <option value="GenericToken">Token Genérico</option>
              <option value="IPv4">Dirección IPv4</option>
              <option value="IPv6">Dirección IPv6</option>
              <option value="Email">Correo Electrónico</option>
              <option value="SpanishDniNie">DNI / NIE</option>
              <option value="CreditCard">Tarjeta de Crédito</option>
              <option value="GuidUuid">GUID / UUID</option>
            </select>
          </div>

          <div class="flex items-center justify-end gap-2 pt-3">
            <button
              type="button"
              on:click={() => (showAddPairModal = false)}
              class="px-4 py-2 bg-slate-100 dark:bg-white/5 text-slate-700 dark:text-slate-300 font-semibold rounded-xl border border-slate-300 dark:border-white/10"
            >
              Cancelar
            </button>
            <button
              type="submit"
              class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 font-bold rounded-xl shadow-md shadow-primary-500/20"
            >
              Guardar Par
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- MODAL CREAR NUEVA BÓVEDA -->
  {#if showCreateVaultModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
        <h3 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <span>➕</span> Crear Nueva Bóveda Lexi
        </h3>
        
        <form on:submit|preventDefault={handleCreateVaultSubmit} class="space-y-3 text-xs">
          <div>
            <label for="create-vault-name-input" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Nombre de la Bóveda (.lexi):</label>
            <input
              id="create-vault-name-input"
              use:autofocus
              type="text"
              bind:value={newVaultName}
              placeholder="ej: clientes_2026.lexi"
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white font-mono focus:ring-1 focus:ring-primary-500"
            />
          </div>

          <div>
            <label for="create-vault-pwd-input" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Contraseña de Cifrado (Obligatoria):</label>
            <input
              id="create-vault-pwd-input"
              type="password"
              bind:value={newVaultPassword}
              placeholder="Introduce la contraseña de cifrado..."
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white focus:ring-1 focus:ring-primary-500"
            />
          </div>

          <div class="flex items-center justify-end gap-2 pt-3">
            <button
              type="button"
              on:click={() => (showCreateVaultModal = false)}
              class="px-4 py-2 bg-slate-100 dark:bg-white/5 text-slate-700 dark:text-slate-300 font-semibold rounded-xl border border-slate-300 dark:border-white/10"
            >
              Cancelar
            </button>
            <button
              type="submit"
              class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 font-bold rounded-xl shadow-md shadow-primary-500/20"
            >
              Crear Bóveda
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- MODAL RENOMBRAR BÓVEDA -->
  {#if showRenameVaultModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
        <h3 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <span>✏️</span> Renombrar Bóveda
        </h3>
        
        <form on:submit|preventDefault={handleRenameVaultSubmit} class="space-y-3 text-xs">
          <div>
            <label for="rename-vault-input" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Nuevo Nombre:</label>
            <input
              id="rename-vault-input"
              use:autofocus
              type="text"
              bind:value={renameNewVaultName}
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white font-mono focus:ring-1 focus:ring-primary-500"
            />
          </div>

          <div class="flex items-center justify-end gap-2 pt-3">
            <button
              type="button"
              on:click={() => (showRenameVaultModal = false)}
              class="px-4 py-2 bg-slate-100 dark:bg-white/5 text-slate-700 dark:text-slate-300 font-semibold rounded-xl border border-slate-300 dark:border-white/10"
            >
              Cancelar
            </button>
            <button
              type="submit"
              class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 font-bold rounded-xl shadow-md shadow-primary-500/20"
            >
              Guardar
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- MODAL COPIA DE SEGURIDAD BZIP2 -->
  {#if showBackupModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl p-6 max-w-md w-full shadow-2xl space-y-4">
        <h3 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
          <span>📦</span> Copia de Seguridad Completa (Bzip2)
        </h3>
        <p class="text-xs text-slate-500 dark:text-slate-400">Empaqueta la configuración, reglas personalizadas y todas las bóvedas en un único archivo comprimido con algoritmo Bzip2.</p>
        
        <form on:submit|preventDefault={handleCreateBackup} class="space-y-3 text-xs">
          <div>
            <label for="backup-pwd-modal-input" class="block font-medium text-slate-600 dark:text-slate-400 mb-1">Contraseña de Cifrado AES-256 (Opcional):</label>
            <input
              id="backup-pwd-modal-input"
              use:autofocus
              type="password"
              bind:value={backupPassword}
              placeholder="Contraseña para proteger la copia..."
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white focus:ring-1 focus:ring-primary-500"
            />
          </div>

          <div class="flex items-center justify-end gap-2 pt-3">
            <button
              type="button"
              on:click={() => (showBackupModal = false)}
              class="px-4 py-2 bg-slate-100 dark:bg-white/5 text-slate-700 dark:text-slate-300 font-semibold rounded-xl border border-slate-300 dark:border-white/10"
            >
              Cancelar
            </button>
            <button
              type="submit"
              class="px-5 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-bold rounded-xl shadow-md shadow-amber-500/20"
            >
              Crear Archivo Bzip2
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- MODAL DE LOGS Y DEPURACIÓN -->
  {#if showLogsModal}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-4">
      <div in:slide={{ duration: 200 }} class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl max-w-4xl w-full h-[80vh] flex flex-col shadow-2xl">
        <div class="p-4 border-b border-slate-200 dark:border-white/10 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="text-xl">🐞</span>
            <div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-white">Registros de Depuración en Tiempo Real</h3>
              <p class="text-xs text-slate-500 dark:text-slate-400">Telemetría profunda del motor de ofuscación y llamadas Tauri.</p>
            </div>
          </div>
          
          <div class="flex items-center gap-2">
            <button
              on:click={() => openHelp('logs')}
              class="px-2.5 py-1.5 text-xs font-semibold rounded-xl bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-white/10 flex items-center gap-1 transition"
              title="Ver ayuda sobre logs y diagnóstico"
            >
              <span>❔</span> Ayuda
            </button>
            <button
              on:click={toggleDebugMode}
              class="text-xs px-3 py-1.5 rounded-xl border font-semibold transition {debugMode ? 'bg-amber-500/20 border-amber-500/40 text-amber-600 dark:text-amber-400' : 'bg-slate-100 dark:bg-white/5 border-slate-300 dark:border-white/10 text-slate-700 dark:text-slate-300'}"
            >
              {debugMode ? 'Modo DEBUG: ACTIVO' : 'Modo DEBUG: INACTIVO'}
            </button>
            <button on:click={closeLogsModal} class="text-slate-400 hover:text-slate-200 text-lg px-2">✕</button>
          </div>
        </div>

        <div class="p-3 border-b border-slate-200 dark:border-white/5 flex items-center justify-between gap-3 text-xs bg-slate-50 dark:bg-slate-950/60">
          <div class="flex items-center gap-2">
            <span>Nivel:</span>
            <select bind:value={logFilterLevel} class="bg-white dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded px-2 py-1 text-xs">
              <option value="all">Todos</option>
              <option value="debug">DEBUG</option>
              <option value="info">INFO</option>
              <option value="warn">WARN</option>
              <option value="error">ERROR</option>
            </select>
            <input
              type="text"
              bind:value={logSearch}
              placeholder="Buscar en trazas..."
              class="bg-white dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded px-2 py-1 text-xs w-48"
            />
          </div>

          <div class="flex items-center gap-2">
            <button on:click={copyDebugLogs} class="px-2.5 py-1 bg-slate-200 dark:bg-white/10 rounded hover:bg-slate-300 dark:hover:bg-white/20 text-xs font-medium">Copiar</button>
            <button on:click={clearDebugLogs} class="px-2.5 py-1 bg-rose-500/10 border border-rose-500/20 text-rose-600 dark:text-rose-400 rounded text-xs font-medium">Limpiar</button>
          </div>
        </div>

        <div class="flex-1 p-3 overflow-auto bg-slate-950 font-mono text-[11px] text-slate-300 space-y-1 select-text">
          {#each debugLogs.filter(l => (logFilterLevel === 'all' || l.level.toLowerCase() === logFilterLevel) && (!logSearch || l.message.toLowerCase().includes(logSearch.toLowerCase()))) as log}
            <div class="flex items-start gap-2 leading-relaxed hover:bg-white/5 p-0.5 rounded">
              <span class="text-slate-500 shrink-0">{log.timestamp}</span>
              <span class="px-1 rounded text-[10px] font-bold shrink-0 {log.level === 'DEBUG' ? 'bg-cyan-500/20 text-cyan-400' : log.level === 'INFO' ? 'bg-emerald-500/20 text-emerald-400' : log.level === 'WARN' ? 'bg-amber-500/20 text-amber-400' : 'bg-rose-500/20 text-rose-400'}">{log.level}</span>
              <span class="text-slate-400 shrink-0">[{log.target}]</span>
              <span class="text-slate-200 break-all">{log.message}</span>
            </div>
          {:else}
            <div class="p-8 text-center text-slate-500">No hay registros de depuración capturados.</div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- MODAL DE AYUDA CONTEXTUAL Y GUÍA DE USUARIO -->
  {#if activeHelpTab}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
    >
      <button
        type="button"
        class="fixed inset-0 bg-slate-950/80 backdrop-blur-md w-full h-full cursor-default"
        on:click={closeHelp}
        aria-label="Cerrar manual de ayuda"
      ></button>
      <div in:slide={{ duration: 200 }} class="relative z-10 bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl max-w-3xl w-full max-h-[85vh] flex flex-col shadow-2xl overflow-hidden">
        <!-- Cabecera del modal de ayuda -->
        <div class="p-4 border-b border-slate-200 dark:border-white/10 flex items-center justify-between bg-slate-50 dark:bg-slate-950/60">
          <div class="flex items-center gap-2.5">
            <span class="text-xl">📘</span>
            <div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-white">Manual y Ayuda Contextual de LexiShield</h3>
              <p class="text-xs text-slate-500 dark:text-slate-400">Conceptos clave, recomendaciones de seguridad y flujos de trabajo.</p>
            </div>
          </div>
          <button on:click={closeHelp} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-lg px-2 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 transition">✕</button>
        </div>

        <!-- Selector de Tema de Ayuda -->
        <div class="flex flex-wrap items-center gap-1.5 px-4 py-2 border-b border-slate-200 dark:border-white/5 bg-slate-100/50 dark:bg-black/20 text-xs">
          <button
            on:click={() => (activeHelpTab = "studio")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'studio' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            ✨ Studio & Auditoría
          </button>
          <button
            on:click={() => (activeHelpTab = "pairs")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'pairs' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            📑 Pares de Tokens
          </button>
          <button
            on:click={() => (activeHelpTab = "vaults")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'vaults' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            🗄️ Bóvedas Lexi
          </button>
          <button
            on:click={() => (activeHelpTab = "watch")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'watch' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            📋 Portapapeles
          </button>
          <button
            on:click={() => (activeHelpTab = "config")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'config' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            ⚙️ Configuración
          </button>
          <button
            on:click={() => (activeHelpTab = "password_why")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'password_why' ? 'bg-amber-500/20 text-amber-600 dark:text-amber-400 font-bold border border-amber-500/40' : 'text-amber-600/80 dark:text-amber-400/80 hover:text-amber-600 dark:hover:text-amber-300'}"
          >
            🔐 ¿Por qué Contraseña?
          </button>
          <button
            on:click={() => (activeHelpTab = "logs")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'logs' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            🐞 Logs
          </button>
        </div>

        <!-- Contenido Específico del Tema -->
        <div class="flex-1 p-6 overflow-auto text-xs text-slate-700 dark:text-slate-300 space-y-4 select-text leading-relaxed">
          {#if activeHelpTab === "studio"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>✨</span> Cómo proteger texto para usar en Inteligencia Artificial o compartir datos
              </h4>
              <p>El objetivo principal del Studio es permitirte interactuar de forma segura con modelos de lenguaje (como ChatGPT, Claude, Gemini o IAs locales), o compartir documentos con personas que no deben conocer información confidencial (nombres de clientes, DNI, IBAN, contraseñas, direcciones IP o números de tarjeta).</p>
              
              <div class="p-3.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5 space-y-2">
                <p class="font-bold text-slate-900 dark:text-white">Flujo de Trabajo Recomendado:</p>
                <ol class="list-decimal list-inside space-y-1.5 pl-1">
                  <li><strong>Pega o escribe el contenido</strong> en el panel <em>Texto claro</em> (o arrastra y suelta un archivo o carpeta directamente encima).</li>
                  <li><strong>Detección automática:</strong> El motor identifica automáticamente la estructura sintáctica (JSON, XML, Logs o texto plano) sin que tengas que configurarlo.</li>
                  <li><strong>Pulsa "Proteger":</strong> LexiShield reemplazará cada dato confidencial por un seudónimo consistente (por ejemplo, <code class="bg-primary-500/20 px-1 rounded font-mono">Dni_001</code> o <code class="bg-primary-500/20 px-1 rounded font-mono">AnonPerson_001</code>).</li>
                  <li><strong>Usa el texto en la IA:</strong> Copia el <em>Texto protegido</em> y envíalo con total tranquilidad a cualquier modelo de IA o tercero.</li>
                  <li><strong>Restaura la respuesta de la IA:</strong> Cuando la IA responda citando los seudónimos, pega su respuesta en <em>Texto protegido</em> y pulsa <strong>"Restaurar"</strong> para recuperar al 100% los valores reales.</li>
                </ol>
              </div>

              <div class="p-3 bg-primary-500/10 border border-primary-500/20 rounded-xl space-y-1">
                <p class="font-bold text-primary-700 dark:text-primary-300">📄 Conversión a Markdown con Pandoc:</p>
                <p>Si activas la casilla "Exportar Markdown con Pandoc", LexiShield convertirá documentos estructurados (.docx, .html, .rst) a Markdown nativo (.md), conservando encabezados, listas y tablas perfectas para ser interpretadas por modelos de IA con el menor consumo de tokens.</p>
              </div>

              <div class="p-3.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5 space-y-2">
                <p class="font-bold text-slate-900 dark:text-white flex items-center gap-1.5">
                  <span>🔍</span> Desglose y Auditoría de Tokens:
                </p>
                <p>Puedes cambiar a la sub-vista <em>"Desglose y Auditoría"</em> o pulsar <em>"Auditar"</em> en cualquier momento. LexiShield analizará tu texto o carpetas completas sin tocarlas, clasificando los tokens por Red, Identidad, Sistemas o Claves, y permitiéndote omitir falsos positivos antes de guardarlos en tu bóveda.</p>
              </div>
            </div>

          {:else if activeHelpTab === "pairs"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>📑</span> Gestión de Pares de Tokens (Biyectividad 1:1)
              </h4>
              <p>Cada vez que LexiShield detecta o sustituye un token sensible, crea una correspondencia biyectiva en la bóveda activa. Esto garantiza que la misma entidad confidencial siempre reciba exactamente el mismo seudónimo en todos tus documentos.</p>
              
              <div class="space-y-2">
                <p><strong>Operaciones Principales:</strong></p>
                <ul class="list-disc list-inside space-y-1 pl-1">
                  <li><strong>Añadir Par:</strong> Permite precargar asociaciones personalizadas (ej. tu nombre con un seudónimo elegido por ti).</li>
                  <li><strong>Omitir Token:</strong> Si una palabra clave o término técnico no debe sustituirse nunca, selecciónalo y pulsa "Omitir".</li>
                  <li><strong>Restaurar .bak:</strong> Si cometes un error masivo, puedes recuperar la última copia automática de seguridad de la bóveda con un solo clic.</li>
                </ul>
              </div>
            </div>

          {:else if activeHelpTab === "vaults"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>🗄️</span> Gestión de Bóvedas Lexi y Copias Bzip2
              </h4>
              <p>Las bóvedas (<code class="font-mono">.lexi</code>) son archivos binarios herméticos donde residen tus tokens y correspondencias. Puedes crear múltiples bóvedas para separar tus clientes, proyectos o entornos personales.</p>
              
              <div class="p-3.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5 space-y-2">
                <p class="font-bold text-slate-900 dark:text-white">Copias de Seguridad Comprimidas con Bzip2:</p>
                <p>La herramienta de backup empaqueta toda tu carpeta <code class="font-mono">~/.lexishield/</code> (incluyendo reglas TOML personalizadas y todas las bóvedas) en un único archivo comprimido con el estándar Bzip2 y opcionalmente protegido con cifrado AES-256 para almacenamiento seguro fuera de línea.</p>
              </div>
            </div>

          {:else if activeHelpTab === "watch"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>📋</span> Monitorización Activa del Portapapeles (Watch Mode)
              </h4>
              <p>El modo Watch convierte a LexiShield en un guardián silencioso de tu portapapeles. Mientras trabajas, cada vez que copias texto confidencial, el sistema lo transforma automáticamente sin que tengas que abrir y cerrar la aplicación.</p>
              
              <div class="space-y-2">
                <p><strong>Características de Ergonomía y Seguridad:</strong></p>
                <ul class="list-disc list-inside space-y-1 pl-1">
                  <li><strong>Notificación Transparente:</strong> Cuando copias texto sin secretos, se informa claramente de <em>"0 tokens reconocidos"</em> para que sepas que el texto no fue modificado innecesariamente.</li>
                  <li><strong>Vista Previa Inmediata:</strong> El historial muestra en tiempo real tanto el texto capturado como el texto transformado colocado en el portapapeles.</li>
                  <li><strong>Vaciado Seguro:</strong> Puedes pulsar <em>"Vaciar Portapapeles"</em> o activar en Configuración el borrado automático tras N segundos para que ningún secreto permanezca en la memoria del sistema operativo.</li>
                </ul>
              </div>
            </div>

          {:else if activeHelpTab === "config"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>⚙️</span> Configuración y Ergonomía
              </h4>
              <p>LexiShield se adapta a tu flujo de trabajo. Puedes personalizar qué pestañas se muestran en la barra superior si solo usas el Studio o el Escáner, elegir tu pestaña predeterminada al abrir el programa y conmutar la barra de comandos CLI equivalentes en vivo.</p>
              <p class="text-[11px] text-slate-400">Todos los ajustes se guardan siguiendo las mejores prácticas de SO en <code class="font-mono">~/.lexishield/config.toml</code> con capacidad de restablecimiento automático en cualquier momento.</p>
            </div>

          {:else if activeHelpTab === "password_why"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-amber-600 dark:text-amber-400 flex items-center gap-2">
                <span>🔐</span> ¿Por qué LexiShield exige contraseña en sus bóvedas?
              </h4>
              <div class="p-3.5 bg-amber-500/10 border border-amber-500/30 rounded-xl space-y-2 text-slate-800 dark:text-slate-200">
                <p class="font-bold text-slate-900 dark:text-white">Principio Fundamental: Arquitectura de Cero Conocimiento (Zero-Knowledge)</p>
                <p>LexiShield fue diseñado para proteger secretos de nivel empresarial y datos personales de máxima confidencialidad. Una contraseña es obligatoria por tres motivos arquitectónicos innegociables:</p>
                <ul class="list-disc list-inside space-y-1.5 pl-1 text-[11px]">
                  <li><strong>Imposibilidad de Ataques Físicos o Forenses:</strong> Si alguien te roba el equipo o accede a tu disco duro, sin la contraseña maestra es matemáticamente imposible reconstruir la relación entre los seudónimos y los nombres/IPs/secretos reales.</li>
                  <li><strong>Criptografía Argon2id + ChaCha20-Poly1305:</strong> La clave de descifrado no se guarda en ningún fichero ni en memoria persistente; se deriva mediante Argon2id con parámetros intensivos en memoria RAM para neutralizar intentos de fuerza bruta con tarjetas gráficas (GPUs) y clústeres ASIC.</li>
                  <li><strong>Aislamiento Estricto por Proyecto o Cliente:</strong> Al asignar contraseñas distintas a diferentes bóvedas (<code class="font-mono">.lexi</code>), garantizas que cada cliente o entorno mantenga una muralla criptográfica impenetrable.</li>
                </ul>
              </div>
            </div>

          {:else if activeHelpTab === "logs"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>🐞</span> Diagnóstico y Registros de Telemetría
              </h4>
              <p>La consola de logs registra cada evento del motor nativo en Rust (invocaciones a Tauri, lecturas y escrituras de bóvedas, detecciones y transformaciones). Puedes activar el <strong>Modo DEBUG</strong> para trazas detalladas o filtrar por nivel (INFO, WARN, ERROR).</p>
            </div>
          {/if}
        </div>

        <!-- Pie del modal de ayuda -->
        <div class="p-4 border-t border-slate-200 dark:border-white/10 flex items-center justify-end bg-slate-50 dark:bg-slate-950/60">
          <button
            on:click={closeHelp}
            class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-md shadow-primary-500/20"
          >
            Entendido / Cerrar
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- MODAL DE CONFIRMACIÓN Y DETALLE DE ACCIÓN COMPLETADA -->
  {#if actionResultModal}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
    >
      <button
        type="button"
        class="fixed inset-0 bg-slate-950/80 backdrop-blur-md w-full h-full cursor-default"
        on:click={() => (actionResultModal = null)}
        aria-label="Cerrar ventana de confirmación"
      ></button>
      <div in:slide={{ duration: 200 }} class="relative z-10 bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl max-w-lg w-full p-6 flex flex-col shadow-2xl space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl {actionResultModal.type === 'info' ? 'bg-indigo-500/10 border-indigo-500/20 text-indigo-500' : 'bg-emerald-500/10 border-emerald-500/20 text-emerald-500'} flex items-center justify-center text-xl shrink-0">
            {actionResultModal.type === 'info' ? 'ℹ️' : '✅'}
          </div>
          <div>
            <h3 class="text-sm font-bold text-slate-900 dark:text-white">{actionResultModal.title}</h3>
            <p class="text-xs text-slate-500 dark:text-slate-400">{actionResultModal.message}</p>
          </div>
        </div>

        {#if actionResultModal.details && actionResultModal.details.length > 0}
          <div class="p-3 bg-slate-50 dark:bg-slate-950/80 rounded-xl border border-slate-200 dark:border-white/5 space-y-2 text-xs font-mono">
            {#each actionResultModal.details as det}
              <div class="flex items-center justify-between gap-4 py-1 border-b border-slate-200/60 dark:border-white/5 last:border-b-0">
                <span class="text-slate-500 dark:text-slate-400 font-sans text-[11px]">{det.label}:</span>
                <span class="text-slate-900 dark:text-white font-bold truncate select-all text-right" title={det.value}>{det.value}</span>
              </div>
            {/each}
          </div>
        {/if}

        <div class="flex items-center justify-end pt-2">
          <button
            on:click={() => (actionResultModal = null)}
            class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-md shadow-primary-500/20"
          >
            Aceptar
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- MODAL UNIFICADO DE PROCESAMIENTO DE ARCHIVO O CARPETA EN STUDIO -->
  {#if studioDropModal}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
    >
      <button
        type="button"
        class="fixed inset-0 bg-slate-950/80 backdrop-blur-md w-full h-full cursor-default"
        on:click={cancelStudioDropModal}
        aria-label="Cerrar modal de procesamiento"
      ></button>
      <div in:slide={{ duration: 200 }} class="relative z-10 bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl max-w-lg w-full p-6 flex flex-col shadow-2xl space-y-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl {studioDropModal.isReverse ? 'bg-amber-500/10 border-amber-500/20 text-amber-500' : 'bg-primary-500/10 border-primary-500/20 text-primary-500'} flex items-center justify-center text-xl shrink-0">
            {studioDropModal.isDir ? '📁' : '📄'}
          </div>
          <div class="min-w-0 flex-1">
            <h3 class="text-sm font-bold text-slate-900 dark:text-white truncate">
              {studioDropModal.isReverse ? 'Restaurar' : 'Proteger'} {studioDropModal.isDir ? 'Carpeta' : 'Archivo'}: {studioDropModal.name}
            </h3>
            <p class="text-xs text-slate-500 dark:text-slate-400 truncate" title={studioDropModal.path}>{studioDropModal.path}</p>
          </div>
        </div>

        <p class="text-xs text-slate-600 dark:text-slate-300">
          {#if studioDropModal.isReverse}
            Has soltado un elemento en <strong>Texto protegido</strong>. Se restaurarán los datos originales utilizando tu bóveda local de mapeos.
          {:else}
            Has soltado un elemento en <strong>Texto claro</strong>. Se protegerán los tokens confidenciales reemplazándolos por seudónimos seguros.
          {/if}
        </p>

        <div class="p-3.5 bg-slate-50 dark:bg-slate-950/80 rounded-xl border border-slate-200 dark:border-white/5 space-y-3.5 text-xs">
          <!-- Opción Copia Separada y Sufijo -->
          <div class="space-y-2">
            <label class="flex items-center gap-2.5 cursor-pointer text-slate-700 dark:text-slate-300">
              <input type="checkbox" bind:checked={studioDropModal.copyMode} class="rounded accent-primary-500" />
              <div>
                <span class="font-semibold text-slate-900 dark:text-white">Guardar en una copia con sufijo</span>
                <p class="text-[11px] text-slate-400">Conserva el archivo original intacto y genera una copia procesada.</p>
              </div>
            </label>

            {#if studioDropModal.copyMode}
              <div class="pl-6 space-y-2 pt-1 border-l-2 border-primary-500/30 ml-2">
                <div class="flex items-center gap-2">
                  <label for="modal-suffix-input" class="text-[11px] font-semibold text-slate-600 dark:text-slate-400 shrink-0">Sufijo del archivo:</label>
                  <input
                    id="modal-suffix-input"
                    type="text"
                    bind:value={studioDropModal.suffix}
                    placeholder="_obfuscated"
                    class="bg-white dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1 text-slate-900 dark:text-slate-100 font-mono text-xs focus:ring-1 focus:ring-primary-500 focus:outline-none flex-1"
                  />
                </div>

                <div class="p-2 bg-primary-500/10 rounded-lg border border-primary-500/20 text-[11px] font-mono text-primary-700 dark:text-primary-300 flex items-center gap-1.5 truncate">
                  <span class="font-bold">Vista previa:</span>
                  <span class="truncate" title={computeOutputPreview(studioDropModal)}>{computeOutputPreview(studioDropModal)}</span>
                </div>

                <label class="flex items-center gap-2 cursor-pointer text-slate-600 dark:text-slate-400 text-[11px]">
                  <input type="checkbox" bind:checked={studioDropModal.saveSuffixDefault} class="rounded accent-primary-500" />
                  <span>Recordar este sufijo como predeterminado en mi configuración</span>
                </label>
              </div>
            {:else}
              <div class="p-2 bg-amber-500/10 rounded-lg border border-amber-500/20 text-[11px] text-amber-700 dark:text-amber-300 flex items-center gap-1.5">
                <span>⚠️</span>
                <span>Se modificará directamente el archivo original en su ubicación actual.</span>
              </div>
            {/if}
          </div>

          <!-- Opciones específicas de Directorio -->
          {#if studioDropModal.isDir}
            <div class="pt-2 border-t border-slate-200/60 dark:border-white/5">
              <label class="flex items-center gap-2.5 cursor-pointer text-slate-700 dark:text-slate-300">
                <input type="checkbox" bind:checked={studioDropModal.cleanCache} class="rounded accent-primary-500" />
                <div>
                  <span class="font-semibold text-slate-900 dark:text-white">Limpiar cachés de sesión</span>
                  <p class="text-[11px] text-slate-400">Fuerza el re-análisis completo de todos los archivos del directorio sin usar cachés previas.</p>
                </div>
              </label>
            </div>
          {/if}

          <!-- Opción Markdown con Pandoc -->
          {#if hasPandoc}
            <div class="pt-2 border-t border-slate-200/60 dark:border-white/5">
              <label class="flex items-center gap-2.5 cursor-pointer text-slate-700 dark:text-slate-300">
                <input type="checkbox" bind:checked={studioDropModal.exportMarkdown} class="rounded accent-primary-500" />
                <div>
                  <span class="font-semibold text-slate-900 dark:text-white">Guardar en Markdown (.md)</span>
                  <p class="text-[11px] text-slate-400">Convierte automáticamente documentos de Office procesados a formato Markdown con Pandoc.</p>
                </div>
              </label>
            </div>
          {/if}
        </div>

        <div class="flex items-center justify-end gap-2 pt-2">
          <button
            on:click={cancelStudioDropModal}
            class="px-4 py-2 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl transition"
          >
            Cancelar
          </button>
          <button
            on:click={confirmStudioDropModal}
            class="px-5 py-2 {studioDropModal.isReverse ? 'bg-amber-500 hover:bg-amber-400' : 'bg-primary-600 hover:bg-primary-500'} text-slate-950 text-xs font-bold rounded-xl transition shadow-md flex items-center gap-1.5"
          >
            {#if studioDropModal.isReverse}
              <span>🔄</span>
              Restaurar {studioDropModal.isDir ? 'Carpeta' : 'Archivo'}
            {:else}
              <span>🛡️</span>
              Proteger {studioDropModal.isDir ? 'Carpeta' : 'Archivo'}
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- OVERLAY VISUAL DE ARRASTRE DE ARCHIVOS O CARPETAS (DRAG & DROP) -->
  {#if isDragging && allowDragDrop}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-8 pointer-events-none transition animate-in fade-in duration-200">
      {#if activeTab === "studio"}
        <!-- Overlay dividido de Studio: Texto claro (Proteger) y Texto protegido (Restaurar) -->
        <div class="w-full max-w-4xl grid grid-cols-1 md:grid-cols-2 gap-6 min-h-[360px]">
          <!-- Zona Izquierda: Texto claro (Proteger) -->
          <div class="rounded-3xl border-2 border-dashed p-8 flex flex-col items-center justify-center text-center gap-4 transition-all duration-150 {studioDragTarget === 'deobfuscated' ? 'border-primary-400 bg-primary-500/20 scale-[1.02] shadow-2xl ring-4 ring-primary-500/30' : 'border-slate-500/50 bg-slate-900/60 opacity-75'}">
            <div class="w-20 h-20 rounded-2xl bg-primary-500/10 border border-primary-500/30 flex items-center justify-center text-5xl animate-bounce">
              📥
            </div>
            <h3 class="text-xl font-extrabold text-white">
              Panel Texto Claro
            </h3>
            <div class="px-3 py-1 bg-primary-500 text-slate-950 text-xs font-black rounded-full uppercase tracking-wider">
              🛡️ Suelta aquí para PROTEGER
            </div>
            <p class="text-xs text-slate-300 max-w-xs">
              Se protegerán los datos confidenciales (IPs, emails, nombres, claves) por seudónimos seguros. Admite archivos individuales o carpetas completas.
            </p>
          </div>

          <!-- Zona Derecha: Texto protegido (Restaurar) -->
          <div class="rounded-3xl border-2 border-dashed p-8 flex flex-col items-center justify-center text-center gap-4 transition-all duration-150 {studioDragTarget === 'obfuscated' ? 'border-amber-400 bg-amber-500/20 scale-[1.02] shadow-2xl ring-4 ring-amber-500/30' : 'border-slate-500/50 bg-slate-900/60 opacity-75'}">
            <div class="w-20 h-20 rounded-2xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-center text-5xl animate-bounce">
              📤
            </div>
            <h3 class="text-xl font-extrabold text-white">
              Panel Texto Protegido
            </h3>
            <div class="px-3 py-1 bg-amber-400 text-slate-950 text-xs font-black rounded-full uppercase tracking-wider">
              🔄 Suelta aquí para RESTAURAR
            </div>
            <p class="text-xs text-slate-300 max-w-xs">
              Se restaurarán los valores reales originales a partir de los seudónimos utilizando tu bóveda local de mapeos.
            </p>
          </div>
        </div>
      {:else}
        <!-- Overlay del Escáner u otras vistas -->
        <div class="bg-white/90 dark:bg-slate-900/95 border-2 border-dashed border-primary-500 rounded-3xl p-10 flex flex-col items-center gap-4 shadow-2xl text-center max-w-md frosted-card pulse-glow">
          <div class="w-20 h-20 rounded-2xl bg-primary-500/10 border border-primary-500/30 flex items-center justify-center text-5xl animate-bounce">
            📂
          </div>
          <h3 class="text-base font-bold text-slate-900 dark:text-white">
            Suelta tu archivo o carpeta aquí para escanear
          </h3>
          <p class="text-xs text-slate-600 dark:text-slate-300">
            LexiShield auditará los tokens confidenciales sin modificar los archivos originales en disco.
          </p>
        </div>
      {/if}
    </div>
  {/if}

  <!-- MODAL DE ATAJOS DE TECLADO ERGONÓMICOS -->
  {#if showKeyboardShortcutsModal}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
    >
      <button
        type="button"
        class="fixed inset-0 bg-slate-950/80 backdrop-blur-md w-full h-full cursor-default"
        on:click={() => (showKeyboardShortcutsModal = false)}
        aria-label="Cerrar chuleta de atajos"
      ></button>
      <div in:slide={{ duration: 200 }} class="relative z-10 bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl max-w-lg w-full p-6 flex flex-col shadow-2xl space-y-4">
        <div class="flex items-center justify-between pb-3 border-b border-slate-200 dark:border-white/10">
          <div class="flex items-center gap-2.5">
            <span class="text-2xl">⌨️</span>
            <div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-white">Atajos de Teclado de Studio</h3>
              <p class="text-xs text-slate-500 dark:text-slate-400">Opera todo el flujo de protección sin separar las manos del teclado.</p>
            </div>
          </div>
          <button on:click={() => (showKeyboardShortcutsModal = false)} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-lg px-2 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 transition">✕</button>
        </div>

        <div class="grid grid-cols-1 gap-2.5 text-xs font-mono">
          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Proteger o Restaurar datos (según modo activo)</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Ctrl</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Enter</kbd>
            </div>
          </div>

          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Copiar texto resultante al portapapeles</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Ctrl</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Shift</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">C</kbd>
            </div>
          </div>

          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Alternar dirección (Proteger ⇄ Restaurar)</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Ctrl</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Shift</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">X</kbd>
            </div>
          </div>

          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Auditar texto actual sin modificarlo</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Ctrl</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Shift</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">A</kbd>
            </div>
          </div>

          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Limpiar ambos paneles (con guardado en historial)</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Ctrl</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Shift</kbd>
              <span>+</span>
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">L</kbd>
            </div>
          </div>

          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Abrir esta chuleta de atajos rápidos</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">?</kbd>
            </div>
          </div>

          <div class="flex items-center justify-between p-2.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5">
            <span class="font-sans text-slate-700 dark:text-slate-300">Cerrar ventanas y modales activos</span>
            <div class="flex items-center gap-1">
              <kbd class="px-2 py-1 bg-slate-200 dark:bg-slate-800 rounded font-bold border border-slate-300 dark:border-white/10 shadow-xs">Esc</kbd>
            </div>
          </div>
        </div>

        <div class="flex items-center justify-end pt-2">
          <button
            on:click={() => (showKeyboardShortcutsModal = false)}
            class="px-5 py-2 bg-primary-600 hover:bg-primary-500 text-slate-950 text-xs font-bold rounded-xl transition shadow-md shadow-primary-500/20"
          >
            Entendido
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- MODAL DE HISTORIAL TEMPORAL DE SESIÓN (ALTA VISIBILIDAD & ZERO-KNOWLEDGE) -->
  {#if showSessionHistoryModal}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
    >
      <button
        type="button"
        class="fixed inset-0 bg-slate-950/80 backdrop-blur-md w-full h-full cursor-default"
        on:click={() => (showSessionHistoryModal = false)}
        aria-label="Cerrar historial de sesión"
      ></button>
      <div in:slide={{ duration: 200 }} class="relative z-10 bg-white dark:bg-slate-900 border border-slate-200 dark:border-white/10 rounded-2xl max-w-2xl w-full max-h-[85vh] flex flex-col shadow-2xl overflow-hidden">
        <!-- Cabecera -->
        <div class="p-4 border-b border-slate-200 dark:border-white/10 flex items-center justify-between bg-slate-50 dark:bg-slate-950/60">
          <div class="flex items-center gap-3">
            <span class="text-2xl">⏱️</span>
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-sm font-bold text-slate-900 dark:text-white">Historial Temporal de Sesión</h3>
                <span class="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20">
                  🔒 En memoria RAM · Zero-Knowledge
                </span>
              </div>
              <p class="text-xs text-slate-500 dark:text-slate-400">Recupera cualquier texto protegido, restaurado o limpiado en la sesión activa.</p>
            </div>
          </div>
          <button on:click={() => (showSessionHistoryModal = false)} class="text-slate-400 hover:text-slate-600 dark:hover:text-white text-lg px-2 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 transition">✕</button>
        </div>

        <!-- Lista de tarjetas de historial -->
        <div class="flex-1 overflow-auto p-4 space-y-3">
          {#if sessionHistory.length === 0}
            <div class="p-10 border-2 border-dashed border-slate-200 dark:border-white/10 rounded-2xl text-center space-y-2">
              <span class="text-3xl">📭</span>
              <h4 class="text-sm font-bold text-slate-700 dark:text-slate-300">Aún no hay transformaciones en esta sesión</h4>
              <p class="text-xs text-slate-400 max-w-sm mx-auto">
                Cada vez que pulses "Proteger", "Restaurar" o "Limpiar todo", se creará una copia instantánea aquí para que nunca pierdas tu trabajo.
              </p>
            </div>
          {:else}
            {#each sessionHistory as item (item.id)}
              <div class="p-3.5 bg-slate-50 dark:bg-slate-950/70 border border-slate-200 dark:border-white/10 rounded-xl hover:border-primary-500/40 transition flex flex-col gap-2.5 shadow-sm">
                <div class="flex items-center justify-between flex-wrap gap-2">
                  <div class="flex items-center gap-2">
                    {#if item.actionType === 'obfuscate'}
                      <span class="px-2.5 py-1 rounded-lg text-xs font-bold bg-primary-500/20 text-primary-700 dark:text-primary-300 border border-primary-500/30 flex items-center gap-1">
                        <span>🛡️</span> PROTEGIDO
                      </span>
                    {:else if item.actionType === 'deobfuscate'}
                      <span class="px-2.5 py-1 rounded-lg text-xs font-bold bg-indigo-500/20 text-indigo-700 dark:text-indigo-300 border border-indigo-500/30 flex items-center gap-1">
                        <span>🔄</span> RESTAURADO
                      </span>
                    {:else}
                      <span class="px-2.5 py-1 rounded-lg text-xs font-bold bg-amber-500/20 text-amber-700 dark:text-amber-300 border border-amber-500/30 flex items-center gap-1">
                        <span>🗑️</span> ESTADO PREVIO A LIMPIEZA
                      </span>
                    {/if}
                    <span class="font-mono text-xs text-slate-700 dark:text-slate-300 font-semibold">{item.timestamp}</span>
                    <span class="text-[11px] text-slate-400">({formatRelativeTime(item.dateObj)})</span>
                  </div>

                  <div class="flex items-center gap-2">
                    {#if item.tokensCount > 0}
                      <span class="px-2 py-0.5 rounded text-[11px] font-mono font-medium bg-slate-200 dark:bg-white/10 text-slate-700 dark:text-slate-300">
                        {item.tokensCount} tokens
                      </span>
                    {/if}
                    <button
                      on:click={() => restoreHistoryItem(item)}
                      class="px-3.5 py-1.5 bg-primary-600 hover:bg-primary-500 text-slate-950 font-bold text-xs rounded-xl transition shadow-md shadow-primary-500/20 flex items-center gap-1"
                      title="Cargar estos textos directamente en los paneles de Studio"
                    >
                      <span>📥</span> Cargar en Editores
                    </button>
                  </div>
                </div>

                <!-- Extractos de texto contrastados -->
                <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs font-mono">
                  <div class="p-2 bg-white dark:bg-slate-900/90 rounded-lg border border-slate-200 dark:border-white/5 space-y-1">
                    <span class="text-[10px] uppercase font-bold text-slate-400 block">📥 Texto claro:</span>
                    <p class="text-slate-700 dark:text-slate-300 truncate select-all">{item.inputText || "(Vacío)"}</p>
                  </div>
                  <div class="p-2 bg-white dark:bg-slate-900/90 rounded-lg border border-slate-200 dark:border-white/5 space-y-1">
                    <span class="text-[10px] uppercase font-bold text-slate-400 block">🛡️ Texto protegido:</span>
                    <p class="text-primary-700 dark:text-primary-400 truncate select-all">{item.outputText || "(Vacío)"}</p>
                  </div>
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <!-- Pie del modal -->
        <div class="p-3 border-t border-slate-200 dark:border-white/10 flex items-center justify-between bg-slate-50 dark:bg-slate-950/60">
          <button
            on:click={() => {
              sessionHistory = [];
              statusMessage = "Historial temporal de sesión vaciado.";
            }}
            disabled={sessionHistory.length === 0}
            class="px-3 py-1.5 text-xs text-rose-600 dark:text-rose-400 hover:bg-rose-500/10 rounded-xl transition disabled:opacity-40"
          >
            🗑️ Vaciar Historial
          </button>
          <button
            on:click={() => (showSessionHistoryModal = false)}
            class="px-5 py-1.5 bg-slate-200 dark:bg-white/10 hover:bg-slate-300 dark:hover:bg-white/20 text-slate-800 dark:text-slate-200 text-xs font-semibold rounded-xl transition"
          >
            Cerrar
          </button>
        </div>
      </div>
    </div>
  {/if}

</main>
