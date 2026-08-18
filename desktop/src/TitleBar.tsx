import { Minus, Square, X } from "lucide-react";
import { closeWindow, minimizeWindow, toggleMaximizeWindow } from "./window";

export default function TitleBar({ compact = false }: { compact?: boolean }) {
  return (
    <header
      className={`titlebar ${compact ? "compact" : ""}`}
      data-tauri-drag-region
      onDoubleClick={() => {
        if (!compact) void toggleMaximizeWindow();
      }}
    >
      <div className="titlebar-brand" data-tauri-drag-region>
          <span className="titlebar-mark">技</span>
        <span>技术交流</span>
      </div>
      <div className="window-controls">
        <button type="button" aria-label="最小化" title="最小化" onClick={() => void minimizeWindow()}>
          <Minus size={15} strokeWidth={1.7} />
        </button>
        {!compact && (
          <button type="button" aria-label="最大化" title="最大化" onClick={() => void toggleMaximizeWindow()}>
            <Square size={12} strokeWidth={1.6} />
          </button>
        )}
        <button className="close" type="button" aria-label="关闭" title="关闭" onClick={() => void closeWindow()}>
          <X size={15} strokeWidth={1.7} />
        </button>
      </div>
    </header>
  );
}
