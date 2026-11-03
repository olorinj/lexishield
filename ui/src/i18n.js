import { register, init, getLocaleFromNavigator, locale, t } from "svelte-i18n";

register("es", () => import("./locales/es.json"));
register("en", () => import("./locales/en.json"));

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

