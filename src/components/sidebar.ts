import { icons, createElement } from "lucide";

export type PanelId = "scan" | "diagnose";

interface SidebarCallbacks {
  onSwitch: (panel: PanelId) => void;
}

export function initSidebar(container: HTMLElement, callbacks: SidebarCallbacks): () => void {
  const items: { id: PanelId; icon: keyof typeof icons; label: string }[] = [
    { id: "scan", icon: "HardDrive", label: "磁盘扫描" },
    { id: "diagnose", icon: "Activity", label: "系统诊断" },
  ];

  const nav = document.createElement("nav");
  nav.className = "sidebar";

  const btnGroup = document.createElement("div");
  btnGroup.className = "sidebar-btn-group";

  const buttons: HTMLButtonElement[] = [];

  for (const item of items) {
    const btn = document.createElement("button");
    btn.className = "sidebar-btn";
    btn.dataset.panel = item.id;
    btn.title = item.label;

    const iconEl = createElement(icons[item.icon]);
    iconEl.classList.add("sidebar-icon");
    btn.append(iconEl);

    btn.addEventListener("click", () => {
      setActive(item.id);
      callbacks.onSwitch(item.id);
    });

    btnGroup.append(btn);
    buttons.push(btn);
  }

  nav.append(btnGroup);
  container.append(nav);

  function setActive(id: PanelId) {
    for (const btn of buttons) {
      btn.classList.toggle("active", btn.dataset.panel === id);
    }
  }

  return () => {
    container.removeChild(nav);
  };
}
