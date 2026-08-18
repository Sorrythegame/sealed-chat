import { ArrowRight, LoaderCircle, LockKeyhole, ShieldCheck } from "lucide-react";
import { FormEvent, useEffect, useState } from "react";
import { api, setToken } from "./api";
import Chat from "./Chat";
import { crypto } from "./crypto";
import TitleBar from "./TitleBar";
import { setAppWindowMode } from "./window";

type AuthMode = "login" | "register";

export default function App() {
  const [authed, setAuthed] = useState(false);
  const [checking, setChecking] = useState(true);
  const [mode, setMode] = useState<AuthMode>("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [inviteCode, setInviteCode] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function restoreSession() {
      try {
        const token = await crypto.getToken();
        if (cancelled) return;
        if (token) {
          setToken(token);
          await setAppWindowMode("chat");
          if (!cancelled) setAuthed(true);
        } else {
          await setAppWindowMode("auth");
        }
      } catch {
        await setAppWindowMode("auth");
      } finally {
        if (!cancelled) setChecking(false);
      }
    }

    void restoreSession();
    return () => {
      cancelled = true;
    };
  }, []);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const normalizedUsername = username.trim();
    if (!normalizedUsername || !password) {
      setError("请输入用户名和密码");
      return;
    }
    if (mode === "register" && !inviteCode.trim()) {
      setError("请输入邀请码");
      return;
    }

    setError("");
    setBusy(true);
    try {
      const publicIdentity =
        (await crypto.loadIdentity()) ?? (await crypto.generateIdentity()).public_identity;
      const response =
        mode === "register"
          ? await api.register({
              username: normalizedUsername,
              password,
              invite_code: inviteCode.trim(),
              device_name: "desktop",
              public_identity: publicIdentity,
            })
          : await api.login({
              username: normalizedUsername,
              password,
              device_name: "desktop",
              public_identity: publicIdentity,
            });

      setToken(response.token);
      await crypto.saveToken(response.token);
      localStorage.setItem("technology-communication.device_id", String(response.device_id));
      localStorage.setItem("technology-communication.user_id", String(response.user_id));
      localStorage.setItem("technology-communication.username", response.username);
      await setAppWindowMode("chat");
      setAuthed(true);
    } catch (submitError) {
      setError(readableAuthError(submitError, mode));
    } finally {
      setBusy(false);
    }
  }

  async function handleLogout() {
    setAuthed(false);
    setMode("login");
    setPassword("");
    setError("");
    await setAppWindowMode("auth");
  }

  return (
    <div className={`app-window ${authed ? "chat-mode" : "auth-mode"}`}>
      <TitleBar compact={!authed} />
      {checking ? (
        <div className="auth-loading" aria-live="polite">
          <span className="brand-orb small"><ShieldCheck size={30} /></span>
          <LoaderCircle className="spin" size={21} />
          <span>正在安全载入</span>
        </div>
      ) : authed ? (
        <Chat onLogout={handleLogout} />
      ) : (
        <main className="auth-page">
          <div className="auth-brand">
            <span className="brand-orb"><ShieldCheck size={36} strokeWidth={1.7} /></span>
            <h1>技术交流</h1>
            <p><LockKeyhole size={13} /> 端到端加密沟通</p>
          </div>

          <form className="auth-form" onSubmit={submit}>
            <div className="auth-tabs" role="tablist" aria-label="账号操作">
              <button
                type="button"
                role="tab"
                aria-selected={mode === "login"}
                className={mode === "login" ? "active" : ""}
                onClick={() => {
                  setMode("login");
                  setError("");
                }}
              >
                登录
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={mode === "register"}
                className={mode === "register" ? "active" : ""}
                onClick={() => {
                  setMode("register");
                  setError("");
                }}
              >
                注册
              </button>
            </div>

            <label className="auth-field">
              <span>用户名</span>
              <input
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                autoComplete="username"
                autoFocus
                placeholder="请输入用户名"
              />
            </label>
            <label className="auth-field">
              <span>密码</span>
              <input
                type="password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                autoComplete={mode === "login" ? "current-password" : "new-password"}
                placeholder="请输入密码"
              />
            </label>

            {mode === "register" && (
              <label className="auth-field">
                <span>邀请码</span>
                <input
                  value={inviteCode}
                  onChange={(event) => setInviteCode(event.target.value)}
                  autoComplete="off"
                  spellCheck={false}
                  placeholder="JSJL-XXXXX-XXXXX-XXXXX-XXXXX"
                />
              </label>
            )}

            {error && <div className="auth-error" role="alert">{error}</div>}

            <button className="auth-submit" type="submit" disabled={busy}>
              {busy ? <LoaderCircle className="spin" size={18} /> : <ArrowRight size={18} />}
              {busy ? "处理中" : mode === "login" ? "进入消息" : "创建账号"}
            </button>
          </form>
          <p className="auth-footnote">消息内容仅在通信双方的设备上解密</p>
        </main>
      )}
    </div>
  );
}

function readableAuthError(error: unknown, mode: AuthMode) {
  const message = String(error);
  if (message.includes("401")) return "用户名或密码不正确";
  if (message.includes("429")) return "操作过于频繁，请稍后再试";
  if (message.includes("409")) return "该用户名已被注册";
  if (message.includes("invite code")) return "邀请码无效或已失效";
  if (message.includes("Failed to fetch")) return "暂时无法连接服务器，请稍后重试";
  return mode === "login" ? "登录失败，请检查输入后重试" : "注册失败，请稍后重试";
}
