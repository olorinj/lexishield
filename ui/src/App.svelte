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
  let activeTab = "studio"; // "studio" | "scanner" | "pairs" | "vaults" | "watch" | "config"

  // Configuración de la Aplicación y Ergonomía
  let appConfig = {
    gui: {
      default_tab: "studio",
      show_cli_dock: true,
      tabs: {
        show_studio: true,
        show_scanner: true,
        show_pairs: true,
        show_vaults: true,
        show_watch: true,
      },
      clipboard: {
        notification_type: "toast",
        auto_clear_enabled: false,
        auto_clear_seconds: 5,
      },
    },
  };

  // Estado del Studio
  let inputText = "";
  let outputText = "";
  let selectedFormat = "auto";
  let vaultPath = "";
  let password = "";
  let loading = false;
  let statusMessage = "Listo. Pega tu texto o selecciona un archivo para procesarlo.";
  let lastReport = null;
  let vaultMappings = [];

  // Opciones de Ofuscación
  let copyMode = true; // Por defecto guardar en copia sufijada
  let copySuffix = "_obfuscated";
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

  // Estado de Ayuda Contextual y Onboarding
  let activeHelpTab = null; // "studio" | "scanner" | "pairs" | "vaults" | "watch" | "config" | "logs" | "password_why"
  let dismissedTips = JSON.parse(localStorage.getItem("dismissed_tips") || "{}");

  // Estado de Notificación / Modal de Acción Completada
  let actionResultModal = null; // { title: string, message: string, details?: Array<{label: string, value: string}>, type?: "success" | "info" }

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
        if (event.payload.type === "enter" || event.payload.type === "over") {
          isDragging = true;
        } else if (event.payload.type === "leave") {
          isDragging = false;
        } else if (event.payload.type === "drop") {
          isDragging = false;
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            await handleDroppedPath(paths[0]);
          }
        }
      });
    } catch (err) {
      console.warn("Listener de Drag & Drop de Tauri no disponible:", err);
    }

    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
    if (unlistenDragDrop) unlistenDragDrop();
    if (scanPollTimer) clearInterval(scanPollTimer);
    if (watchPollInterval) clearInterval(watchPollInterval);
    if (logPollingInterval) clearInterval(logPollingInterval);
    if (autoClearTimer) clearTimeout(autoClearTimer);
  });

  function handleKeydown(e) {
    if (e.key === "Escape") {
      if (activeHelpTab) {
        closeHelp();
      } else if (actionResultModal) {
        actionResultModal = null;
      }
    }
  }

  function toggleTheme() {
    darkMode = !darkMode;
    localStorage.setItem("theme", darkMode ? "dark" : "light");
    updateThemeClass();
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
        appConfig = cfg;
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
    }
  }

  async function handleInitVaultSubmit() {
    const pwd = initVaultPassword.trim();
    if (!pwd) {
      alert("⚠️ Es obligatorio proporcionar una contraseña para inicializar y proteger la bóveda.");
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
    }
  }

  async function handleUnlockVaultSubmit() {
    if (!unlockPassword) {
      unlockError = "Por favor ingrese la contraseña de la bóveda";
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
      statusMessage = "Por favor ingresa texto para ofuscar.";
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
      statusMessage = `✅ Ofuscación exitosa: ${res.report.replacements_applied} tokens reemplazados en ${res.report.elapsed_ms.toFixed(2)}ms`;
      actionResultModal = {
        title: "Ofuscación Finalizada con Éxito",
        type: "success",
        message: `El texto ha sido transformado para proteger tus datos confidenciales antes de enviarlo a la IA o compartirlo con terceros.`,
        details: [
          { label: "Tokens Reemplazados", value: `${res.report.replacements_applied}` },
          { label: "Tiempo de Procesamiento", value: `${res.report.elapsed_ms.toFixed(2)} ms` },
          { label: "Formato Aplicado", value: selectedFormat.toUpperCase() },
          { label: "Bóveda Destino", value: vaultPath ? vaultPath.split('/').pop() : "Predeterminada" }
        ]
      };
      refreshVaultsList();
    } catch (err) {
      statusMessage = "Error al ofuscar: " + err;
    } finally {
      loading = false;
    }
  }

  async function handleDeobfuscate() {
    if (!inputText.trim()) {
      statusMessage = "Por favor ingresa texto para desofuscar.";
      return;
    }
    loading = true;
    try {
      const res = await invokeTauri("deobfuscate_content", {
        text: inputText,
        formatStr: selectedFormat,
        vaultPath: vaultPath || null,
        password: password || null,
        customMappings: null
      });
      outputText = res.deobfuscated_text;
      lastReport = res.report;
      statusMessage = `✅ Desofuscación exitosa: ${res.report.replacements_applied} tokens restaurados en ${res.report.elapsed_ms.toFixed(2)}ms`;
      actionResultModal = {
        title: "Desofuscación Finalizada",
        type: "success",
        message: "Las respuestas de la IA han sido desofuscadas reconstruyendo los valores originales mediante tu bóveda local.",
        details: [
          { label: "Tokens Restaurados", value: `${res.report.replacements_applied}` },
          { label: "Tiempo de Proceso", value: `${res.report.elapsed_ms.toFixed(2)} ms` }
        ]
      };
    } catch (err) {
      statusMessage = "Error al desofuscar: " + err;
    } finally {
      loading = false;
    }
  }

  async function processFilePathInStudio(selected) {
    if (!selected) return;
    try {
      loading = true;
      let outPath = selected;
      if (copyMode) {
        const dotIdx = selected.lastIndexOf(".");
        if (dotIdx !== -1) {
          outPath = selected.substring(0, dotIdx) + copySuffix + selected.substring(dotIdx);
        } else {
          outPath = selected + copySuffix;
        }
      }

      const rep = await invokeTauri("process_file_command", {
        inputPath: selected,
        outputPath: outPath,
        formatStr: selectedFormat,
        vaultPath: vaultPath || null,
        password: password || null,
        reverse: false,
        exportMd: hasPandoc && exportMarkdown
      });
      lastReport = rep;
      statusMessage = `✅ Archivo procesado (${rep.replacements_applied} tokens reemplazados). Guardado en: ${outPath}`;
      actionResultModal = {
        title: "Archivo Ofuscado con Éxito",
        type: "success",
        message: `El archivo ha sido procesado y guardado con los tokens confidenciales protegidos.`,
        details: [
          { label: "Archivo Origen", value: selected.split('/').pop() },
          { label: "Archivo Guardado", value: outPath.split('/').pop() },
          { label: "Tokens Reemplazados", value: `${rep.replacements_applied}` },
          { label: "Tiempo", value: `${rep.elapsed_ms.toFixed(2)} ms` },
          { label: "Exportado a Markdown", value: (hasPandoc && exportMarkdown) ? "Sí (.md)" : "No" }
        ]
      };
      refreshVaultsList();
    } catch (err) {
      statusMessage = "Error al procesar archivo: " + err;
    } finally {
      loading = false;
    }
  }

  async function openFileToProcess() {
    try {
      const selected = await open({
        filters: [
          { name: "Documentos y Logs", extensions: ["txt", "log", "json", "xml", "csv", "tsv", "docx", "xlsx", "pptx", "md", "markdown"] },
          { name: "Todos los archivos", extensions: ["*"] }
        ],
        multiple: false
      });
      if (selected) {
        await processFilePathInStudio(selected);
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
      activeTab = "scanner";
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
      activeTab = "scanner";
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

  async function scanPastedText() {
    if (!textToAudit.trim()) return;
    try {
      loading = true;
      detectedItems = [];
      auditSummary = null;
      activeTab = "scanner";

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

      statusMessage = `🔍 Texto analizado: ${res.total_entities} tokens confidenciales detectados.`;
      actionResultModal = {
        title: "Análisis de Texto Finalizado",
        type: "info",
        message: `Se han identificado los tokens confidenciales del fragmento copiado.`,
        details: [
          { label: "Origen", value: "Texto pegado en Escáner" },
          { label: "Tokens Detectados", value: `${res.total_entities}` },
          { label: "Longitud de Caracteres", value: `${textToAudit.length}` }
        ]
      };
    } catch (err) {
      statusMessage = "Error al analizar texto: " + err;
    } finally {
      loading = false;
    }
  }

  function sendPastedTextToStudio() {
    if (!textToAudit.trim()) return;
    inputText = textToAudit;
    activeTab = "studio";
    statusMessage = "📋 Texto transferido a Studio. Listo para ofuscar o desofuscar.";
  }

  async function handleDroppedPath(droppedPath) {
    if (!droppedPath) return;
    try {
      const isDir = await invokeTauri("is_directory", { path: droppedPath });
      if (activeTab === "scanner") {
        if (isDir) {
          await auditDirectoryPath(droppedPath);
        } else {
          await auditFilePath(droppedPath);
        }
      } else if (activeTab === "studio") {
        if (isDir) {
          activeTab = "scanner";
          await auditDirectoryPath(droppedPath);
        } else {
          await processFilePathInStudio(droppedPath);
        }
      } else {
        activeTab = "scanner";
        if (isDir) {
          await auditDirectoryPath(droppedPath);
        } else {
          await auditFilePath(droppedPath);
        }
      }
    } catch (err) {
      statusMessage = "Error al procesar elemento arrastrado: " + err;
    }
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
      return;
    }
    const pwd = newVaultPassword.trim();
    if (!pwd) {
      alert("⚠️ Es obligatorio configurar una contraseña para proteger la nueva bóveda.");
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
      } catch (e) {}
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
      return `lexishield obfuscate -f ${selectedFormat}${vaultArg}${cleanArg}${copyArg}${pwdArg}`;
    } else if (activeTab === "scanner") {
      const recArg = recursiveScan ? " -r" : " --no-recursive";
      const noGitArg = noGitIgnoreScan ? " --no-git-ignore" : "";
      return `lexishield scan -i "<ruta>"${recArg}${noGitArg}${vaultArg}`;
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

      {#if appConfig.gui.tabs.show_scanner}
        <button
          on:click={() => (activeTab = "scanner")}
          class="px-3 py-1.5 rounded-lg transition duration-150 flex items-center gap-1.5 {activeTab === 'scanner' ? 'bg-white dark:bg-primary-500/20 text-slate-950 dark:text-primary-300 border border-slate-200 dark:border-primary-500/40 shadow-sm font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-950 dark:hover:text-white border border-transparent'}"
        >
          <span>🔍</span> Escáner
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
      <!-- VISTA STUDIO -->
      <div in:fade={{ duration: 150 }} class="flex-1 flex flex-col gap-4 min-h-0">
        <!-- Barra de Opciones Superiores del Studio -->
        <div class="bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-3.5 flex flex-wrap items-center justify-between gap-3 shadow-sm text-xs">
          <div class="flex items-center gap-3">
            <label class="font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
              <span>📐 Formato:</span>
              <select bind:value={selectedFormat} class="bg-white dark:bg-slate-900 border border-slate-300 dark:border-white/10 rounded-lg px-2.5 py-1 text-slate-900 dark:text-slate-100 focus:ring-1 focus:ring-primary-500 font-medium">
                <option value="auto" class="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">Auto (Detección inteligente)</option>
                <option value="json" class="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">JSON Estructurado</option>
                <option value="xml" class="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">XML / HTML</option>
                <option value="log" class="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">Log Delimitado / KVP</option>
                <option value="text" class="bg-white dark:bg-slate-900 text-slate-900 dark:text-white">Texto Plano</option>
              </select>
            </label>

            <label class="flex items-center gap-1.5 cursor-pointer text-slate-700 dark:text-slate-300">
              <input type="checkbox" bind:checked={copyMode} class="rounded accent-primary-500" />
              <span>Guardar en copia sufijada ({copySuffix})</span>
            </label>

            <label class="flex items-center gap-1.5 cursor-pointer text-slate-700 dark:text-slate-300">
              <input type="checkbox" bind:checked={cleanCache} class="rounded accent-primary-500" />
              <span>Limpiar cachés de sesión</span>
            </label>

            {#if hasPandoc}
              <label class="flex items-center gap-1.5 cursor-pointer text-slate-700 dark:text-slate-300" title="Requiere Pandoc instalado en el sistema para convertir a Markdown">
                <input type="checkbox" bind:checked={exportMarkdown} class="rounded accent-primary-500" />
                <span>Guardar en Markdown (.md)</span>
              </label>
            {/if}
          </div>

          <div class="flex items-center gap-2">
            <button
              on:click={() => openHelp('studio')}
              class="px-2.5 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-300 rounded-xl border border-primary-500/30 font-bold transition flex items-center gap-1"
              title="Ver guía de uso de Studio"
            >
              <span>❔</span> Ayuda
            </button>
            <button
              on:click={openFileToProcess}
              disabled={loading}
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 rounded-xl border border-slate-300 dark:border-white/10 font-semibold transition flex items-center gap-1.5 disabled:opacity-50"
            >
              <span>📄</span> Procesar Archivo...
            </button>
            <button
              on:click={openVaultFile}
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 rounded-xl border border-slate-300 dark:border-white/10 font-semibold transition flex items-center gap-1.5"
            >
              <span>📂</span> Cambiar Bóveda
            </button>
          </div>
        </div>

        {#if !dismissedTips.studio}
          <div in:slide={{ duration: 150 }} class="p-3.5 bg-gradient-to-r from-primary-500/10 via-brand-surface/40 to-primary-500/5 rounded-2xl border border-primary-500/20 flex items-center justify-between gap-3 text-xs shadow-sm">
            <div class="flex items-center gap-3">
              <span class="text-lg">✨</span>
              <p class="text-slate-700 dark:text-slate-300">
                <strong class="text-primary-600 dark:text-primary-400">¿Cómo usar Studio?</strong> Ofusca texto para usarlo en motores de Inteligencia Artificial (p. ej. asistentes en la nube o LLMs locales) u ocúltalo de quien no deba conocer datos confidenciales (IPs, emails, nombres, claves). Al recibir la respuesta de la IA, usa <em>Desofuscar Original</em> para restaurar los valores reales.
              </p>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <button on:click={() => openHelp('studio')} class="text-[11px] font-bold text-primary-600 dark:text-primary-400 hover:underline">Ver detalles</button>
              <button on:click={() => dismissTip('studio')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white px-1">✕</button>
            </div>
          </div>
        {/if}

        <!-- Paneles de Entrada y Salida -->
        <div class="flex-1 grid grid-cols-1 md:grid-cols-2 gap-4 min-h-0">
          <!-- Entrada -->
          <div class="bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-4 flex flex-col min-h-0 shadow-sm">
            <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-white/5 mb-2">
              <span class="text-xs font-bold text-slate-700 dark:text-slate-300 uppercase tracking-wider flex items-center gap-1.5">
                <span>📥</span> Entrada Original
              </span>
              <button
                on:click={() => {
                  inputText = "";
                  outputText = "";
                  statusMessage = "Área de trabajo de Studio limpiada.";
                }}
                class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 text-xs transition"
                title="Limpiar tanto la entrada original como el resultado procesado"
              >
                Limpiar
              </button>
            </div>
            <textarea
              bind:value={inputText}
              placeholder="Pega aquí el contenido a ofuscar o desofuscar (JSON, Logs, XML, código)..."
              class="flex-1 w-full bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 p-3 text-xs font-mono text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-primary-500 resize-none overflow-auto select-text"
            ></textarea>
          </div>

          <!-- Salida -->
          <div class="bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-4 flex flex-col min-h-0 shadow-sm">
            <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-white/5 mb-2">
              <span class="text-xs font-bold text-slate-700 dark:text-slate-300 uppercase tracking-wider flex items-center gap-1.5">
                <span>📤</span> Resultado Procesado
              </span>
              <div class="flex items-center gap-2">
                <button
                  on:click={() => { outputText = ""; }}
                  disabled={!outputText}
                  class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 text-xs transition disabled:opacity-40"
                  title="Limpiar solo el resultado"
                >
                  Limpiar
                </button>
                <button
                  on:click={() => { navigator.clipboard.writeText(outputText); statusMessage = "📋 Copiado al portapapeles."; }}
                  disabled={!outputText}
                  class="text-primary-600 dark:text-primary-400 hover:underline text-xs font-semibold disabled:opacity-40 transition"
                >
                  Copiar
                </button>
              </div>
            </div>
            <textarea
              bind:value={outputText}
              readonly
              placeholder="El resultado sanitizado o desofuscado aparecerá aquí..."
              class="flex-1 w-full bg-slate-50 dark:bg-slate-950/60 rounded-xl border border-slate-200 dark:border-white/5 p-3 text-xs font-mono text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none resize-none overflow-auto select-text"
            ></textarea>
          </div>
        </div>

        <!-- Botonera de Acción Principal del Studio -->
        <div class="flex items-center justify-end gap-3">
          <button
            on:click={handleDeobfuscate}
            disabled={loading || !inputText.trim()}
            class="px-5 py-2 bg-slate-200 hover:bg-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-900 dark:text-white text-xs font-bold rounded-xl border border-slate-300 dark:border-white/15 transition duration-150 flex items-center gap-2 disabled:opacity-50 shadow-sm"
          >
            <span>🔄</span> Desofuscar Original
          </button>
          <button
            on:click={handleObfuscate}
            disabled={loading || !inputText.trim()}
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
      <!-- VISTA ESCÁNER -->
      <div in:fade={{ duration: 150 }} class="flex-1 bg-white dark:bg-brand-surface/80 backdrop-blur-md rounded-2xl border border-slate-200 dark:border-white/10 p-6 flex flex-col min-h-0 shadow-lg">
        <div class="flex flex-wrap items-center justify-between gap-4 mb-4">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
              <span>🔍</span> Escáner de Entidades Confidenciales
            </h2>
            <p class="text-xs text-slate-500 dark:text-slate-400">Escanea archivos o carpetas completas sin modificarlos. Desmarca las casillas si deseas omitir falsos positivos.</p>
          </div>
          
          <div class="flex items-center gap-3">
            <button
              on:click={() => openHelp('scanner')}
              class="px-2.5 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-300 rounded-xl border border-primary-500/30 text-xs font-bold transition flex items-center gap-1 shadow-sm"
              title="Ver guía de uso del Escáner"
            >
              <span>❔</span> Ayuda
            </button>

            <label class="flex items-center gap-1.5 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
              <input type="checkbox" bind:checked={recursiveScan} class="rounded accent-primary-500" />
              <span>Recursivo (subdirectorios)</span>
            </label>

            <button
              on:click={() => (showTextScanPanel = !showTextScanPanel)}
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition flex items-center gap-1.5 shadow-sm"
              title="Copiar y pegar texto para auditar tokens confidenciales"
            >
              <span>📋</span> Analizar Texto
            </button>
            <button
              on:click={openFileToAudit}
              disabled={loading}
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 dark:bg-white/5 dark:hover:bg-white/10 text-slate-700 dark:text-slate-300 text-xs font-semibold rounded-xl border border-slate-300 dark:border-white/10 transition flex items-center gap-1.5 shadow-sm disabled:opacity-50"
            >
              <span>📄</span> Escanear Fichero
            </button>
            <button
              on:click={openDirectoryToAudit}
              disabled={loading}
              class="px-3 py-1.5 bg-primary-500/10 hover:bg-primary-500/20 text-primary-700 dark:text-primary-400 text-xs font-semibold rounded-xl border border-primary-500/30 transition flex items-center gap-1.5 shadow-sm disabled:opacity-50"
            >
              <span>📁</span> Escanear Carpeta
            </button>
          </div>
        </div>

        {#if !dismissedTips.scanner}
          <div in:slide={{ duration: 150 }} class="mb-4 p-3.5 bg-gradient-to-r from-primary-500/10 via-brand-surface/40 to-primary-500/5 rounded-2xl border border-primary-500/20 flex items-center justify-between gap-3 text-xs shadow-sm">
            <div class="flex items-center gap-3">
              <span class="text-lg">🔍</span>
              <p class="text-slate-700 dark:text-slate-300">
                <strong class="text-primary-600 dark:text-primary-400">¿Cómo funciona el Escáner?</strong> Inspecciona archivos, carpetas o texto copiado en busca de tokens confidenciales (DNI, tarjetas, IPv4, emails, GUIDs) sin modificar el disco. Puedes omitir falsos positivos desmarcando casillas antes de registrar los hallazgos en tu bóveda.
              </p>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <button on:click={() => openHelp('scanner')} class="text-[11px] font-bold text-primary-600 dark:text-primary-400 hover:underline">Ver detalles</button>
              <button on:click={() => dismissTip('scanner')} class="text-slate-400 hover:text-slate-600 dark:hover:text-white px-1">✕</button>
            </div>
          </div>
        {/if}

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
              class="w-full h-28 bg-white dark:bg-slate-950/80 rounded-xl border border-slate-200 dark:border-white/10 p-3 text-xs font-mono text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-primary-500 resize-none overflow-auto select-text"
            ></textarea>
            <div class="flex items-center justify-between flex-wrap gap-2">
              <span class="text-[11px] text-slate-500 dark:text-slate-400">
                Se identificarán tokens confidenciales y se cargarán en la tabla inferior sin tocar el disco.
              </span>
              <div class="flex items-center gap-2">
                {#if textToAudit.trim()}
                  <button
                    on:click={sendPastedTextToStudio}
                    class="px-3 py-1.5 bg-slate-200 hover:bg-slate-300 dark:bg-white/10 dark:hover:bg-white/20 text-slate-800 dark:text-slate-200 text-xs font-semibold rounded-xl transition flex items-center gap-1"
                    title="Transferir este texto a Studio para ofuscarlo directamente"
                  >
                    <span>➡️</span> Enviar a Studio
                  </button>
                {/if}
                <button
                  on:click={scanPastedText}
                  disabled={loading || !textToAudit.trim()}
                  class="px-4 py-1.5 bg-primary-600 hover:bg-primary-700 text-white text-xs font-bold rounded-xl shadow-sm transition disabled:opacity-50 flex items-center gap-1.5"
                >
                  <span>🔍</span> Analizar Tokens
                </button>
              </div>
            </div>
          </div>
        {/if}

        {#if detectedItems.length === 0 && !isScanningDirectory && !showTextScanPanel}
          <div class="mb-4 p-8 border-2 border-dashed border-slate-300 dark:border-white/15 rounded-2xl flex flex-col items-center justify-center text-center gap-2 bg-slate-50/50 dark:bg-slate-900/30 transition hover:border-primary-500/50">
            <span class="text-4xl mb-1">📂</span>
            <p class="text-xs font-bold text-slate-800 dark:text-slate-200">
              Arrastra y suelta aquí cualquier archivo o carpeta
            </p>
            <p class="text-[11px] text-slate-500 dark:text-slate-400 max-w-md">
              LexiShield detectará automáticamente si es un archivo individual o un directorio para iniciar la auditoría de tokens confidenciales.
            </p>
            <div class="flex items-center gap-2 mt-2">
              <button on:click={openFileToAudit} class="text-xs text-primary-600 dark:text-primary-400 hover:underline font-semibold">Seleccionar archivo</button>
              <span class="text-slate-400 text-xs">•</span>
              <button on:click={openDirectoryToAudit} class="text-xs text-primary-600 dark:text-primary-400 hover:underline font-semibold">Seleccionar carpeta</button>
              <span class="text-slate-400 text-xs">•</span>
              <button on:click={() => (showTextScanPanel = true)} class="text-xs text-primary-600 dark:text-primary-400 hover:underline font-semibold">Pegar texto</button>
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
          <!-- Barra de Acciones del Escáner (Simetría CLI/GUI) -->
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
          <table class="w-full text-left text-xs font-mono">
            <thead class="bg-slate-200 dark:bg-slate-900/90 text-slate-700 dark:text-slate-400 sticky top-0 border-b border-slate-300 dark:border-white/10">
              <tr>
                <th class="p-3 w-12 text-center">
                  <input
                    type="checkbox"
                    checked={detectedItems.length > 0 && selectedScanOriginals.length === detectedItems.length}
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
              {#each detectedItems as item (item.original)}
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
                    <span class="px-2 py-0.5 rounded bg-primary-500/10 text-primary-700 dark:text-primary-400 text-[11px] border border-primary-500/20">
                      {item.detector_type}
                    </span>
                  </td>
                  <td class="p-3 text-amber-600 dark:text-amber-300 font-semibold select-text">{item.original}</td>
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
                      <div class="font-medium text-slate-600 dark:text-slate-300">No hay entidades escaneadas todavía</div>
                      <div class="text-xs text-slate-400 mt-1">Usa los botones superiores "Escanear Fichero" o "Escanear Carpeta".</div>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
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
                  <td class="p-3 text-amber-600 dark:text-amber-300 font-semibold select-text">{item.original}</td>
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
                      <p class="truncate text-slate-700 dark:text-slate-300 select-all" title={ev.originalText}>{ev.originalText}</p>
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
                          <span class="text-amber-600 dark:text-amber-400 font-semibold">{tok.original}</span>
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
                  <option value="scanner">🔍 Escáner</option>
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
                  <input type="checkbox" bind:checked={appConfig.gui.tabs.show_scanner} class="rounded accent-primary-500" />
                  <span>Escáner</span>
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

  <!-- BARRA DE ESTADO INFERIOR -->
  <footer class="h-7 bg-slate-200 dark:bg-brand-surface border-t border-slate-300 dark:border-white/5 px-4 flex items-center justify-between text-[11px] text-slate-600 dark:text-slate-400 shrink-0 select-text">
    <div class="flex items-center gap-2 truncate">
      <span class="truncate">{statusMessage}</span>
    </div>
    <div class="flex items-center gap-3 shrink-0 font-mono text-[10px]">
      {#if lastReport}
        <span>Reemplazos: {lastReport.replacements_applied}</span>
        <span>Tiempo: {lastReport.elapsed_ms.toFixed(1)}ms</span>
      {/if}
      <span>Bóveda: {vaultPath ? vaultPath.split('/').pop() : 'default.lexi'}</span>
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
              class="w-full bg-slate-50 dark:bg-slate-950 border border-slate-300 dark:border-white/10 rounded-xl px-3 py-2 text-slate-900 dark:text-white font-mono focus:ring-1 focus:ring-primary-500"
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
            ✨ Studio (IA)
          </button>
          <button
            on:click={() => (activeHelpTab = "scanner")}
            class="px-2.5 py-1 rounded-lg font-medium transition {activeHelpTab === 'scanner' ? 'bg-primary-500/20 text-primary-600 dark:text-primary-300 font-bold border border-primary-500/30' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white'}"
          >
            🔍 Escáner
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
                <span>✨</span> Cómo ofuscar texto para usar en Inteligencia Artificial o proteger secretos
              </h4>
              <p>El objetivo principal del Studio es permitirte interactuar de forma segura con modelos de lenguaje (como ChatGPT, Claude, Gemini o IAs locales), o compartir documentos con personas que no deben conocer información confidencial (nombres de clientes, DNI, IBAN, contraseñas, direcciones IP o números de tarjeta).</p>
              
              <div class="p-3.5 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5 space-y-2">
                <p class="font-bold text-slate-900 dark:text-white">Flujo de Trabajo Recomendado:</p>
                <ol class="list-decimal list-inside space-y-1.5 pl-1">
                  <li><strong>Pega el texto confidencial</strong> en el panel de entrada (o abre un archivo con <em>Abrir Archivo</em>).</li>
                  <li><strong>Selecciona el formato:</strong> Si es código estructurado, elige JSON, XML o Logs para garantizar que no se alteren comillas ni delimitadores clave.</li>
                  <li><strong>Pulsa "Ofuscar":</strong> LexiShield reemplazará cada token confidencial por un seudónimo consistente (por ejemplo, <code class="bg-primary-500/20 px-1 rounded font-mono">Dni_001</code> o <code class="bg-primary-500/20 px-1 rounded font-mono">AnonPerson_001</code>).</li>
                  <li><strong>Usa el texto en la IA:</strong> Copia el resultado ofuscado y envíalo con total tranquilidad a cualquier modelo de IA.</li>
                  <li><strong>Restaura la respuesta de la IA:</strong> Cuando la IA responda citando los seudónimos, pega su respuesta en el Studio y pulsa <strong>"Desofuscar"</strong> para recuperar al 100% los valores reales.</li>
                </ol>
              </div>

              <div class="p-3 bg-primary-500/10 border border-primary-500/20 rounded-xl space-y-1">
                <p class="font-bold text-primary-700 dark:text-primary-300">📄 Conversión a Markdown con Pandoc:</p>
                <p>Si activas la casilla "Exportar Markdown con Pandoc", LexiShield convertirá documentos estructurados (.docx, .html, .rst) a Markdown nativo (.md), conservando encabezados, listas y tablas perfectas para ser interpretadas por modelos de IA con el menor consumo de tokens.</p>
              </div>
            </div>

          {:else if activeHelpTab === "scanner"}
            <div class="space-y-3">
              <h4 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <span>🔍</span> Escáner y Auditoría Preventiva de Fugas
              </h4>
              <p>El Escáner analiza proactivamente archivos individuales o proyectos enteros sin modificarlos, informándote de cuántos tokens sensibles están expuestos antes de publicar código o compartir repositorios.</p>
              
              <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div class="p-3 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5 space-y-1">
                  <p class="font-bold text-slate-900 dark:text-white">Escaneo con .gitignore:</p>
                  <p>Por defecto, el motor respeta los archivos <code class="font-mono">.gitignore</code> y omite carpetas masivas como <code class="font-mono">node_modules</code>, <code class="font-mono">target</code>, <code class="font-mono">.git</code> o archivos binarios.</p>
                </div>
                <div class="p-3 bg-slate-50 dark:bg-slate-950/70 rounded-xl border border-slate-200 dark:border-white/5 space-y-1">
                  <p class="font-bold text-slate-900 dark:text-white">Telemetría en Vivo:</p>
                  <p>Durante escaneos de grandes directorios, verás el contador de archivos procesados, entidades encontradas y el archivo que se está examinando en ese instante.</p>
                </div>
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

  <!-- OVERLAY VISUAL DE ARRASTRE DE ARCHIVOS O CARPETAS (DRAG & DROP) -->
  {#if isDragging}
    <div class="fixed inset-0 bg-slate-950/80 backdrop-blur-md z-50 flex items-center justify-center p-8 pointer-events-none transition animate-in fade-in duration-200">
      <div class="bg-white dark:bg-slate-900 border-2 border-dashed border-primary-500 rounded-3xl p-10 flex flex-col items-center gap-4 shadow-2xl text-center max-w-md">
        <span class="text-6xl animate-bounce">📂</span>
        <h3 class="text-base font-bold text-slate-900 dark:text-white">
          {#if activeTab === "studio"}
            Suelta tu archivo aquí para procesarlo en Studio
          {:else}
            Suelta tu archivo o carpeta aquí para escanear
          {/if}
        </h3>
        <p class="text-xs text-slate-600 dark:text-slate-300">
          {#if activeTab === "studio"}
            Se ofuscarán los tokens confidenciales respetando el formato original y guardando en copia sufijada.
          {:else}
            LexiShield auditará los tokens confidenciales sin modificar los archivos originales en disco.
          {/if}
        </p>
      </div>
    </div>
  {/if}

</main>
