import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "技术交流 - 安全、克制的沟通工具",
  description:
    "技术交流是一款端到端加密的桌面沟通工具，提供 Windows 与 macOS 客户端下载。",
};

const WINDOWS_DOWNLOAD =
  "/downloads/technology-communication_0.1.1_x64-setup.exe";
const MAC_APPLE_SILICON_DOWNLOAD =
  "/downloads/technology-communication_0.1.1_aarch64.dmg";
const MAC_INTEL_DOWNLOAD =
  "/downloads/technology-communication_0.1.1_x64.dmg";

type IconProps = {
  className?: string;
};

function BrandMark({ className = "" }: IconProps) {
  return <span className={`brand-mark ${className}`}>技</span>;
}

function WindowsIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 5.2 10.7 4v7.35H3V5.2Zm8.8-1.35L21 2.5v8.85h-9.2v-7.5ZM3 12.55h7.7V20L3 18.8v-6.25Zm8.8 0H21v8.95l-9.2-1.35v-7.6Z" />
    </svg>
  );
}

function AppleIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="M16.75 12.55c.02-2 1.64-2.97 1.72-3.02a3.69 3.69 0 0 0-2.9-1.57c-1.22-.13-2.4.73-3.02.73-.64 0-1.59-.72-2.62-.7a3.86 3.86 0 0 0-3.25 1.98c-1.4 2.42-.36 5.98.98 7.94.67.96 1.45 2.03 2.48 1.99 1.01-.04 1.39-.64 2.6-.64 1.2 0 1.56.64 2.61.62 1.08-.02 1.76-.96 2.4-1.93a7.9 7.9 0 0 0 1.1-2.24 3.48 3.48 0 0 1-2.1-3.16Zm-1.98-5.88a3.55 3.55 0 0 0 .82-2.57 3.65 3.65 0 0 0-2.36 1.22 3.4 3.4 0 0 0-.85 2.48c.9.07 1.82-.45 2.39-1.13Z" />
    </svg>
  );
}

function ArrowDownIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 3v12m0 0 4.5-4.5M12 15l-4.5-4.5M5 20h14" />
    </svg>
  );
}

function ShieldIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 3 5.5 5.7v5.6c0 4.15 2.68 7.86 6.5 9.2 3.82-1.34 6.5-5.05 6.5-9.2V5.7L12 3Z" />
      <path d="m9.3 11.8 1.75 1.75 3.8-3.8" />
    </svg>
  );
}

function DatabaseIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <ellipse cx="12" cy="5.5" rx="7" ry="3" />
      <path d="M5 5.5v6c0 1.66 3.13 3 7 3s7-1.34 7-3v-6M5 11.5v6c0 1.66 3.13 3 7 3s7-1.34 7-3v-6" />
    </svg>
  );
}

function ImageIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <rect x="3" y="4" width="18" height="16" rx="3" />
      <circle cx="8.5" cy="9" r="1.5" />
      <path d="m5.5 17 4.2-4.2 3.1 3.1 2.2-2.2 3.5 3.3" />
    </svg>
  );
}

function CheckIcon({ className = "" }: IconProps) {
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true">
      <path d="m5 12.5 4.2 4.2L19 7" />
    </svg>
  );
}

function AppPreview() {
  return (
    <div className="app-preview" aria-label="技术交流客户端界面预览">
      <div className="preview-titlebar">
        <div className="preview-brand">
          <BrandMark className="tiny" />
          <span>技术交流</span>
        </div>
        <div className="preview-controls" aria-hidden="true">
          <i />
          <i />
          <i />
        </div>
      </div>
      <div className="preview-body">
        <aside className="preview-rail">
          <span className="preview-avatar">技</span>
          <span className="rail-dot active" />
          <span className="rail-dot" />
          <span className="rail-dot" />
          <span className="rail-lock">⌁</span>
        </aside>
        <aside className="preview-list">
          <div className="preview-search">搜索联系人</div>
          <div className="preview-contact active">
            <span className="contact-avatar blue">林</span>
            <span><b>林澈</b><small>方案已经发过去了</small></span>
          </div>
          <div className="preview-contact">
            <span className="contact-avatar slate">陈</span>
            <span><b>陈屿</b><small>[图片]</small></span>
          </div>
          <div className="preview-contact">
            <span className="contact-avatar warm">周</span>
            <span><b>周禾</b><small>收到，稍后同步</small></span>
          </div>
        </aside>
        <section className="preview-chat">
          <header>
            <div><span className="contact-avatar blue small">林</span><b>林澈</b></div>
            <span className="secure-pill"><ShieldIcon /> 端到端加密</span>
          </header>
          <div className="preview-messages">
            <div className="message-row">
              <span className="contact-avatar blue small">林</span>
              <span className="bubble">今天的技术方案确认了吗？</span>
            </div>
            <div className="message-row mine">
              <span className="bubble">已确认，我把最终版本发你。</span>
              <span className="contact-avatar dark small">我</span>
            </div>
            <div className="message-row">
              <span className="contact-avatar blue small">林</span>
              <span className="bubble">好，内容很清楚。</span>
            </div>
          </div>
          <div className="preview-composer">
            <div className="composer-tools"><i /><i /></div>
            <span>输入消息，按 Enter 发送</span>
            <button type="button" tabIndex={-1}>发送</button>
          </div>
        </section>
      </div>
    </div>
  );
}

export default function Home() {
  return (
    <main>
      <header className="site-header">
        <a className="brand" href="#top" aria-label="技术交流首页">
          <BrandMark />
          <span>技术交流</span>
        </a>
        <nav aria-label="主导航">
          <a href="#security">安全</a>
          <a href="#download">下载</a>
          <a className="nav-download" href={WINDOWS_DOWNLOAD} download>
            Windows 下载
          </a>
        </nav>
      </header>

      <section className="hero" id="top">
        <div className="hero-copy">
          <div className="trust-label"><ShieldIcon /> 端到端加密桌面沟通</div>
          <h1>让重要沟通，<br /><span>更安静，也更安全。</span></h1>
          <p>
            技术交流是一款简洁、克制的桌面沟通工具。消息只在通信双方设备上解密，
            让每一次讨论都回归内容本身。
          </p>
          <div className="hero-actions">
            <a className="button primary" href={WINDOWS_DOWNLOAD} download>
              <WindowsIcon />
              <span><b>下载 Windows 版</b><small>v0.1.1 · 64 位</small></span>
              <ArrowDownIcon className="arrow" />
            </a>
            <a className="button secondary" href="#download">
              查看全部版本
            </a>
          </div>
          <div className="hero-note">
            <span><CheckIcon /> Windows 10 / 11</span>
            <span><CheckIcon /> 无广告</span>
            <span><CheckIcon /> 免费下载</span>
          </div>
        </div>
        <div className="hero-visual">
          <span className="visual-glow" aria-hidden="true" />
          <AppPreview />
          <div className="floating-card card-encryption">
            <ShieldIcon />
            <span><b>端到端加密</b><small>服务端仅保存密文</small></span>
          </div>
          <div className="floating-card card-status">
            <i /> 安全连接已建立
          </div>
        </div>
      </section>

      <section className="security-section" id="security">
        <div className="section-heading">
          <span>为真实交流而设计</span>
          <h2>安全，不应该成为使用负担</h2>
          <p>从消息到本地记录，安全机制自然融入每一次操作。</p>
        </div>
        <div className="feature-grid">
          <article>
            <span className="feature-icon"><ShieldIcon /></span>
            <h3>端到端加密</h3>
            <p>消息在发送前完成加密，仅通信双方设备可以解密和阅读。</p>
          </article>
          <article>
            <span className="feature-icon"><DatabaseIcon /></span>
            <h3>本地加密缓存</h3>
            <p>历史消息以加密形式保存在本地，兼顾阅读体验与隐私保护。</p>
          </article>
          <article>
            <span className="feature-icon"><ImageIcon /></span>
            <h3>图文与截图</h3>
            <p>支持文字、图片和截图发送，常用沟通能力保持简单直接。</p>
          </article>
        </div>
      </section>

      <section className="download-section" id="download">
        <div className="download-copy">
          <span>桌面客户端</span>
          <h2>选择你的系统</h2>
          <p>Windows 与 macOS 客户端现已提供下载，请根据设备芯片选择对应版本。</p>
        </div>
        <div className="download-grid">
          <article className="platform-card available">
            <div className="platform-top">
              <span className="platform-icon windows"><WindowsIcon /></span>
              <span className="status ready">可下载</span>
            </div>
            <h3>Windows</h3>
            <p>适用于 Windows 10 / 11，64 位系统。</p>
            <dl>
              <div><dt>版本</dt><dd>0.1.1</dd></div>
              <div><dt>格式</dt><dd>EXE 安装程序</dd></div>
            </dl>
            <a className="platform-download" href={WINDOWS_DOWNLOAD} download>
              <WindowsIcon /> 下载 Windows 版 <ArrowDownIcon />
            </a>
            <small>安装程序暂未进行商业代码签名，系统可能显示安全提醒。</small>
          </article>

          <article className="platform-card available" aria-labelledby="mac-title">
            <div className="platform-top">
              <span className="platform-icon apple"><AppleIcon /></span>
              <span className="status ready">可下载</span>
            </div>
            <h3 id="mac-title">macOS</h3>
            <p>适用于 macOS，分别提供 Apple 芯片与 Intel 芯片版本。</p>
            <dl>
              <div><dt>版本</dt><dd>0.1.1</dd></div>
              <div><dt>格式</dt><dd>DMG 安装镜像</dd></div>
            </dl>
            <div className="platform-downloads">
              <a className="platform-download" href={MAC_APPLE_SILICON_DOWNLOAD} download>
                <AppleIcon /> Apple 芯片版 <ArrowDownIcon />
              </a>
              <a className="platform-download secondary-download" href={MAC_INTEL_DOWNLOAD} download>
                <AppleIcon /> Intel 芯片版 <ArrowDownIcon />
              </a>
            </div>
            <small>安装包暂未进行 Apple 公证，首次打开时系统可能显示安全提醒。</small>
          </article>
        </div>
      </section>

      <section className="closing-section">
        <BrandMark className="large" />
        <div>
          <span>技术交流</span>
          <h2>专注交流，保护交流。</h2>
        </div>
        <a className="button primary compact" href={WINDOWS_DOWNLOAD} download>
          <WindowsIcon /> 立即下载
        </a>
      </section>

      <footer>
        <a className="brand footer-brand" href="#top">
          <BrandMark className="tiny" />
          <span>技术交流</span>
        </a>
        <p>端到端加密桌面沟通工具</p>
        <span>© 2026 技术交流</span>
      </footer>
    </main>
  );
}
