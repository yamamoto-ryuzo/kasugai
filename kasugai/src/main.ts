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

    for (const [index, config] of configs.entries()) {
      const tabId = `tab-${index}`;

      // Create tab button
      const button = document.createElement("button");
      button.className = "tab-button";
      button.textContent = config.name;
      button.dataset.tab = tabId;
      tabsContainer.appendChild(button);

      // Create WebView window
      await invoke("create_webview_window", {
        label: tabId,
        url: config.url,
      });

      if (index === 0) {
        button.classList.add("active");
        await invoke("show_window", { label: tabId });
      }
    }

    const tabs = document.querySelectorAll(".tab-button");

    tabs.forEach((tab) => {
      tab.addEventListener("click", async () => {
        const target = (tab as HTMLElement).dataset.tab;
        if (!target) return;

        // Hide all windows
        tabs.forEach(async (t) => {
          const tabId = (t as HTMLElement).dataset.tab;
          if (tabId) {
            await invoke("hide_window", { label: tabId });
          }
        });
        
        // Deactivate all tabs
        tabs.forEach((t) => t.classList.remove("active"));

        // Activate clicked tab and show corresponding window
        tab.classList.add("active");
        await invoke("show_window", { label: target });
      });
    });
  } catch (error) {
    console.error("Failed to load tab configuration:", error);
    contentContainer.textContent = `Error loading configuration: ${error}`;
  }
});
