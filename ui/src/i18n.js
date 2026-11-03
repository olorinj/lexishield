import { register, init, getLocaleFromNavigator, locale, t } from "svelte-i18n";

register("es", () => import("./locales/es.json"));
register("en", () => import("./locales/en.json"));
register("fr", () => import("./locales/fr.json"));
register("de", () => import("./locales/de.json"));
register("pt", () => import("./locales/pt.json"));
register("it", () => import("./locales/it.json"));
register("zh", () => import("./locales/zh.json"));
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

