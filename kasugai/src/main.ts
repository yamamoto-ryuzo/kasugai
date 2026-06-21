import { invoke } from "@tauri-apps/api/core";

interface TabConfig {
  name: string;
  url: string;
}

window.addEventListener("DOMContentLoaded", async () => {
  const tabsContainer = document.getElementById("tabs-container");
  const contentContainer = document.getElementById("content-container");

  if (!tabsContainer || !contentContainer) return;

  try {
    const configs: TabConfig[] = await invoke("get_tabs_config");

    configs.forEach((config, index) => {
      // Create tab button
      const button = document.createElement("button");
      button.className = "tab-button";
      button.textContent = config.name;
      button.dataset.tab = `tab-${index}`;
      tabsContainer.appendChild(button);

      // Create iframe
      const iframe = document.createElement("iframe");
      iframe.id = `tab-${index}`;
      iframe.className = "tab-pane";
      iframe.src = config.url;
      contentContainer.appendChild(iframe);

      if (index === 0) {
        button.classList.add("active");
        iframe.classList.add("active");
      }
    });

    const tabs = document.querySelectorAll(".tab-button");
    const panes = document.querySelectorAll(".tab-pane");

    tabs.forEach((tab) => {
      tab.addEventListener("click", () => {
        const target = (tab as HTMLElement).dataset.tab;

        tabs.forEach((t) => t.classList.remove("active"));
        tab.classList.add("active");

        panes.forEach((pane) => {
          if (pane.id === target) {
            pane.classList.add("active");
          } else {
            pane.classList.remove("active");
          }
        });
      });
    });
  } catch (error) {
    console.error("Failed to load tab configuration:", error);
    contentContainer.textContent = `Error loading configuration: ${error}`;
  }
});
