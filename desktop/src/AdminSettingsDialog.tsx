import { Check, Copy, KeyRound, LoaderCircle, Settings2, X } from "lucide-react";
import { useState } from "react";
import { api, InviteCreateResponse } from "./api";

interface AdminSettingsDialogProps {
  onClose: () => void;
}

export default function AdminSettingsDialog({ onClose }: AdminSettingsDialogProps) {
  const [invite, setInvite] = useState<InviteCreateResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");

  async function generateInvite() {
    setBusy(true);
    setError("");
    setCopied(false);
    try {
      setInvite(await api.createInvite());
    } catch (generateError) {
      const message = String(generateError);
      setError(message.includes("403") ? "当前账号没有邀请码管理权限" : "邀请码生成失败，请稍后重试");
    } finally {
      setBusy(false);
    }
  }

  async function copyInvite() {
    if (!invite) return;
    try {
      await navigator.clipboard.writeText(invite.code);
      setCopied(true);
    } catch {
      setError("自动复制失败，请手动选择邀请码复制");
    }
  }

  return (
    <div
      className="modal-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onClose();
      }}
    >
      <section className="profile-dialog admin-settings-dialog" role="dialog" aria-modal="true" aria-labelledby="admin-settings-title">
        <button className="modal-close" type="button" aria-label="关闭管理设置" onClick={onClose} disabled={busy}>
          <X size={17} />
        </button>

        <div className="admin-settings-heading">
          <span><Settings2 size={21} /></span>
          <div>
            <small>管理设置</small>
            <h2 id="admin-settings-title">邀请码管理</h2>
          </div>
        </div>

        <div className="invite-description">
          <KeyRound size={19} />
          <div>
            <strong>一次性注册邀请码</strong>
            <p>每个邀请码仅可注册一个账号，生成后 30 天内有效。</p>
          </div>
        </div>

        {invite && (
          <div className="invite-result" aria-live="polite">
            <span>新邀请码</span>
            <div>
              <code>{invite.code}</code>
              <button type="button" onClick={() => void copyInvite()} title="复制邀请码">
                {copied ? <Check size={15} /> : <Copy size={15} />}
                {copied ? "已复制" : "复制"}
              </button>
            </div>
            <small>有效期至 {formatExpiry(invite.expires_at)}。明文关闭后无法再次查看。</small>
          </div>
        )}

        {error && <div className="inline-error" role="alert">{error}</div>}

        <button className="profile-save invite-generate" type="button" disabled={busy} onClick={() => void generateInvite()}>
          {busy ? <LoaderCircle className="spin" size={17} /> : <KeyRound size={17} />}
          {busy ? "正在生成" : invite ? "再生成一个" : "生成邀请码"}
        </button>
      </section>
    </div>
  );
}

function formatExpiry(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}
