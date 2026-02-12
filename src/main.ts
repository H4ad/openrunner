import { createApp } from "vue";
import { createPinia } from "pinia";
import * as Sentry from "@sentry/electron/renderer";
import App from "./App.vue";
import "./style.css";

// Initialize Sentry for renderer process
Sentry.init({
  dsn: "https://ef5067aac9111e8769d07679e969a8e1@o4505726434541568.ingest.us.sentry.io/4510872960499712",
  enableLogs: true,
  integrations: [
    Sentry.browserTracingIntegration(),
    Sentry.replayIntegration({
      maskAllText: true,
      blockAllMedia: true,
    }),
  ],
  // Performance monitoring
  tracesSampleRate: 1.0,
  // Session replay sampling
  replaysSessionSampleRate: 0.05,
  replaysOnErrorSampleRate: 1.0,
});

const app = createApp(App);
app.use(createPinia());
app.mount("#app");
