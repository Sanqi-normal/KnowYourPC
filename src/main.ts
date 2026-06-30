import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { initSidebar } from "./components/sidebar";
import { initScanPanel } from "./panels/scan-panel";
import { initDiagnosePanel } from "./panels/diagnose-panel";

// Pre-cache hardware info on startup so panel opens instantly
invoke("get_hardware_info").catch(() => {});

const app = document.getElementById("app")!;
app.innerHTML = `
  <div id="sidebarContainer"></div>
  <main id="panelContainer" class="panel-container"></main>
`;

const sidebarContainer = document.getElementById("sidebarContainer")!;
const panelContainer = document.getElementById("panelContainer")!;

type PanelId = "scan" | "diagnose";
let currentCleanup: (() => void) | null = null;

const panelInit: Record<PanelId, (el: HTMLElement) => () => void> = {
  scan: initScanPanel,
  diagnose: initDiagnosePanel,
};

function switchPanel(id: PanelId) {
  if (currentCleanup) {
    currentCleanup();
    currentCleanup = null;
  }
  panelContainer.replaceChildren();
  currentCleanup = panelInit[id](panelContainer);
}

initSidebar(sidebarContainer, {
  onSwitch: (id) => switchPanel(id),
});

switchPanel("scan");
