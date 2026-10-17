import { createApp } from "vue";
import App from "./App.vue";
import { systemLocale } from "./api";
import { i18n, initLocale } from "./i18n";
import "./style.css";

await initLocale(systemLocale);
createApp(App).use(i18n).mount("#app");
