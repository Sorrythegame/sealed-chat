import { Camera, ImagePlus, LoaderCircle, SendHorizontal } from "lucide-react";
import Image from "@tiptap/extension-image";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { useEffect, useRef, useState } from "react";
import { crypto } from "./crypto";

export type ComposerBlock =
  | { type: "text"; text: string }
  | { type: "image"; bytes: Uint8Array<ArrayBuffer>; mimeType: string };

interface ComposerProps {
  onSend: (blocks: ComposerBlock[]) => Promise<void>;
  disabled?: boolean;
}

export default function Composer({ onSend, disabled = false }: ComposerProps) {
  const fileRef = useRef<HTMLInputElement>(null);
  const [sending, setSending] = useState(false);
  const isDisabled = disabled || sending;

  const editor = useEditor({
    extensions: [StarterKit, Image],
    content: "",
    editable: !disabled,
    editorProps: {
      attributes: {
        class: "composer-editor",
        "data-placeholder": "输入消息，按 Enter 发送",
      },
    },
  });

  useEffect(() => {
    editor?.setEditable(!disabled);
  }, [disabled, editor]);

  function insertImage(bytes: Uint8Array<ArrayBuffer>, mimeType: string) {
    const blob = new Blob([bytes], { type: mimeType });
    const url = URL.createObjectURL(blob);
    editor?.chain().focus().setImage({ src: url }).run();
  }

  async function onPickImage(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) return;
    const buffer = await file.arrayBuffer();
    insertImage(new Uint8Array(buffer), file.type || "image/png");
    if (fileRef.current) fileRef.current.value = "";
  }

  async function onScreenshot() {
    const bytes = await crypto.screenshot();
    insertImage(new Uint8Array(bytes), "image/png");
  }

  async function extractBlocks(): Promise<ComposerBlock[]> {
    if (!editor) return [];
    const document = editor.getJSON();
    const blocks: ComposerBlock[] = [];

    for (const node of document.content ?? []) {
      if (node.type === "image") {
        const src = node.attrs?.src as string | undefined;
        if (!src) continue;
        const response = await fetch(src);
        const blob = await response.blob();
        const buffer = await blob.arrayBuffer();
        blocks.push({
          type: "image",
          bytes: new Uint8Array(buffer),
          mimeType: blob.type || "image/png",
        });
        continue;
      }

      const text = collectText(node);
      if (text.trim()) blocks.push({ type: "text", text });
    }
    return blocks;
  }

  async function handleSend() {
    if (isDisabled) return;
    const blocks = await extractBlocks();
    if (!blocks.length) return;

    setSending(true);
    try {
      await onSend(blocks);
      editor?.commands.clearContent();
    } finally {
      setSending(false);
    }
  }

  function handleKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      void handleSend();
    }
  }

  return (
    <section className="composer" onKeyDown={handleKeyDown}>
      <div className="composer-toolbar">
        <button type="button" onClick={() => void onScreenshot()} disabled={isDisabled} title="截图插入">
          <Camera size={19} strokeWidth={1.7} />
          <span>截图</span>
        </button>
        <button type="button" onClick={() => fileRef.current?.click()} disabled={isDisabled} title="插入图片">
          <ImagePlus size={19} strokeWidth={1.7} />
          <span>图片</span>
        </button>
        <span className="composer-tip">Enter 发送 · Shift + Enter 换行</span>
      </div>
      <EditorContent editor={editor} />
      <div className="composer-footer">
        <span>内容将在本机加密后发送</span>
        <button className="send-button" type="button" onClick={() => void handleSend()} disabled={isDisabled}>
          {sending ? <LoaderCircle className="spin" size={16} /> : <SendHorizontal size={16} />}
          {sending ? "发送中" : "发送"}
        </button>
      </div>
      <input ref={fileRef} hidden type="file" accept="image/*" onChange={onPickImage} />
    </section>
  );
}

function collectText(node: { text?: string; content?: unknown[] }): string {
  let output = node.text || "";
  if (node.content) {
    for (const child of node.content) output += collectText(child as { text?: string; content?: unknown[] });
  }
  return output;
}
