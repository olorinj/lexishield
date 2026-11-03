import "./app.css";
import { setupI18n } from "./i18n";
import App from "./App.svelte";

setupI18n();

const app = new App({
  target: document.getElementById("app"),
});

export default app;

