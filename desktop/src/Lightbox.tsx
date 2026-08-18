import { useEffect, useRef, useState } from "react";

interface LightboxProps {
  src: string;
  onClose: () => void;
}

export default function Lightbox({ src, onClose }: LightboxProps) {
  const [scale, setScale] = useState(1);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const drag = useRef<{ startX: number; startY: number; originX: number; originY: number } | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  function onWheel(e: React.WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY < 0 ? 0.1 : -0.1;
    setScale((s) => Math.min(8, Math.max(0.2, s + delta)));
  }

  function onMouseDown(e: React.MouseEvent) {
    drag.current = { startX: e.clientX, startY: e.clientY, originX: pos.x, originY: pos.y };
  }

  function onMouseMove(e: React.MouseEvent) {
    if (!drag.current) return;
    setPos({
      x: drag.current.originX + (e.clientX - drag.current.startX),
      y: drag.current.originY + (e.clientY - drag.current.startY),
    });
  }

  function onMouseUp() {
    drag.current = null;
  }

  return (
    <div className="lightbox" onClick={onClose}>
      <img
        src={src}
        alt=""
        draggable={false}
        style={{ transform: `translate(${pos.x}px, ${pos.y}px) scale(${scale})` }}
        onClick={(e) => e.stopPropagation()}
        onWheel={onWheel}
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseUp}
      />
      <div className="lightbox-hint">滚轮缩放 · 拖拽移动 · 点击或 ESC 关闭</div>
    </div>
  );
}
