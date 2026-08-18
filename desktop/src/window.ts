import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

export type AppWindowMode = "auth" | "chat";

const AUTH_SIZE = new LogicalSize(380, 520);
const CHAT_SIZE = new LogicalSize(1050, 700);
const CHAT_MIN_SIZE = new LogicalSize(820, 560);

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

export async function setAppWindowMode(mode: AppWindowMode) {
  if (!isTauriRuntime()) return;

  const appWindow = getCurrentWindow();
  try {
    if (mode === "chat") {
      await appWindow.setResizable(true);
      await appWindow.setMaximizable(true);
      await appWindow.setMinSize(CHAT_MIN_SIZE);
      await appWindow.setSize(CHAT_SIZE);
      await appWindow.center();
      return;
    }

    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize();
    }
    await appWindow.setMinSize(null);
    await appWindow.setSize(AUTH_SIZE);
    await appWindow.setMaximizable(false);
    await appWindow.setResizable(false);
    await appWindow.center();
  } catch (error) {
    console.warn(`Failed to switch window to ${mode} mode`, error);
  }
}

export async function minimizeWindow() {
  if (!isTauriRuntime()) return;
  await getCurrentWindow().minimize().catch(console.warn);
}

export async function toggleMaximizeWindow() {
  if (!isTauriRuntime()) return;
  await getCurrentWindow().toggleMaximize().catch(console.warn);
}

export async function closeWindow() {
  if (!isTauriRuntime()) return;
  await getCurrentWindow().close().catch(console.warn);
}
