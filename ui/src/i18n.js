import { register, init, getLocaleFromNavigator, locale, t, addMessages } from "svelte-i18n";
import esJson from "./locales/es.json";
import enJson from "./locales/en.json";
import frJson from "./locales/fr.json";
import deJson from "./locales/de.json";
import itJson from "./locales/it.json";
import zhJson from "./locales/zh.json";

// Precargar de forma síncrona los 6 idiomas principales para garantizar
// disponibilidad inmediata en WebView2 (Windows) y navegadores web
addMessages("es", esJson);
addMessages("en", enJson);
addMessages("fr", frJson);
addMessages("de", deJson);
addMessages("it", itJson);
addMessages("zh", zhJson);

// Registro dinámico para idiomas secundarios/adicionales
register("pt", () => import("./locales/pt.json"));
register("ja", () => import("./locales/ja.json"));
register("ru", () => import("./locales/ru.json"));
register("pl", () => import("./locales/pl.json"));
register("nl", () => import("./locales/nl.json"));
register("ko", () => import("./locales/ko.json"));

export function setupI18n(initialLocale = null) {
  let fallback = "en";
  let detected = initialLocale;

  if (!detected) {
    const nav = getLocaleFromNavigator();
    if (nav && nav.toLowerCase().startsWith("es")) {
      detected = "es";
    } else {
      detected = "en";
    }
  }

  init({
    fallbackLocale: fallback,
    initialLocale: detected,
  });
}

export { locale, t };

