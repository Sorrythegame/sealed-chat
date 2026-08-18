import {
  AlertCircle,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  MessageCircle,
  Plus,
  Search,
  Settings2,
  ShieldCheck,
  UserRound,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { api, ConversationInfo, ProfileInfo, ProfileUpdate, setToken, UserInfo } from "./api";
import AdminSettingsDialog from "./AdminSettingsDialog";
import { DEFAULT_AVATAR } from "./avatar";
import Composer, { ComposerBlock } from "./Composer";
import { crypto } from "./crypto";
import Lightbox from "./Lightbox";
import ProfileDialog from "./ProfileDialog";
import { storage } from "./storage";

type Block =
  | { type: "text"; text: string }
  | { type: "image"; attachment_id: string; key_ciphertext: string; key_nonce: string };
type MessageContent = { blocks: Block[] };

interface ImageRef {
  attachment_id: string;
  key_ciphertext: string;
  key_nonce: string;
}

interface DisplayBlock {
  kind: "text" | "image";
  text?: string;
  image?: ImageRef;
  imageUrl?: string;
}

interface DisplayMessage {
  message_id: number;
  conversation_id: number;
  mine: boolean;
  blocks: DisplayBlock[];
  createdAt?: string;
}

interface ConversationSummary {
  preview: string;
  timestamp: string;
  sortTime: number;
}

interface ChatProps {
  onLogout: () => void | Promise<void>;
}

export default function Chat({ onLogout }: ChatProps) {
  const [profile, setProfile] = useState<ProfileInfo | null>(null);
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [conversations, setConversations] = useState<ConversationInfo[]>([]);
  const [summaries, setSummaries] = useState<Record<number, ConversationSummary>>({});
  const [activeConv, setActiveConv] = useState<ConversationInfo | null>(null);
  const [messages, setMessages] = useState<DisplayMessage[]>([]);
  const [search, setSearch] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [selectingId, setSelectingId] = useState<number | null>(null);
  const [openingUserId, setOpeningUserId] = useState<number | null>(null);
  const [error, setError] = useState("");
  const [lightbox, setLightbox] = useState<string | null>(null);
  const [profileOpen, setProfileOpen] = useState(false);
  const [peerProfileOpen, setPeerProfileOpen] = useState(false);
  const [profileBusy, setProfileBusy] = useState(false);
  const [profileError, setProfileError] = useState("");
  const [adminSettingsOpen, setAdminSettingsOpen] = useState(false);

  const sessionKeys = useRef<Map<number, string>>(new Map());
  const selectionSequence = useRef(0);
  const searchAreaRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const messagesRef = useRef<HTMLDivElement>(null);
  const deviceId = Number(localStorage.getItem("technology-communication.device_id") || 0);

  const orderedConversations = useMemo(
    () => sortConversations(conversations, summaries),
    [conversations, summaries]
  );

  const filteredUsers = useMemo(() => {
    const query = search.trim().toLowerCase();
    return users.filter((user) => !query || user.username.toLowerCase().includes(query));
  }, [search, users]);

  const peerProfile = useMemo<UserInfo | null>(() => {
    if (!activeConv) return null;
    return (
      users.find((user) => user.user_id === activeConv.peer_user_id) ?? {
        user_id: activeConv.peer_user_id,
        username: activeConv.peer_username,
        avatar: activeConv.peer_avatar,
        bio: "",
      }
    );
  }, [activeConv, users]);

  useEffect(() => {
    let cancelled = false;

    async function initialize() {
      setLoading(true);
      try {
        const [profileResult, userResult, conversationResult] = await Promise.all([
          loadProfileWithLegacyFallback(),
          api.listUsers(),
          api.listConversations(),
        ]);
        const normalizedUsers = userResult.users.map((user) => ({ ...user, bio: user.bio || "" }));
        const initialSummaries = await loadConversationSummaries(conversationResult.conversations);
        if (cancelled) return;

        setProfile({ ...profileResult, bio: profileResult.bio || "" });
        setUsers(normalizedUsers);
        setConversations(conversationResult.conversations);
        setSummaries(initialSummaries);

        const firstConversation = sortConversations(
          conversationResult.conversations,
          initialSummaries
        )[0];
        if (firstConversation) void selectConversation(firstConversation);
      } catch (loadError) {
        if (!cancelled) setError(readableError(loadError));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }

    void initialize();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!searchOpen) return;
    const closeOnOutsideClick = (event: PointerEvent) => {
      if (!searchAreaRef.current?.contains(event.target as Node)) setSearchOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setSearchOpen(false);
    };
    window.addEventListener("pointerdown", closeOnOutsideClick);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeOnOutsideClick);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [searchOpen]);

  useEffect(() => {
    const element = messagesRef.current;
    if (!element) return;
    requestAnimationFrame(() => {
      element.scrollTop = element.scrollHeight;
    });
  }, [activeConv?.conversation_id, messages]);

  async function ensureSessionKey(conv: ConversationInfo): Promise<string> {
    const cached = sessionKeys.current.get(conv.conversation_id);
    if (cached) return cached;

    const stored = await storage.loadSessionKey(conv.conversation_id);
    if (stored) {
      sessionKeys.current.set(conv.conversation_id, stored);
      return stored;
    }

    if (!conv.ephemeral_pub) throw new Error("会话密钥缺失");
    const devices = await api.getDevices(conv.peer_user_id);
    const peerPublicKey = devices.devices[0]?.public_identity;
    if (!peerPublicKey) throw new Error("无法获取对方公钥");

    const key = await crypto.completeSession(peerPublicKey, conv.ephemeral_pub);
    sessionKeys.current.set(conv.conversation_id, key);
    await storage.saveSessionKey(conv.conversation_id, key);
    return key;
  }

  async function decryptRecord(
    key: string,
    record: {
      message_id: number;
      conversation_id: number;
      sender_device_id: number;
      ciphertext: string;
      nonce: string;
      created_at: string;
    }
  ): Promise<DisplayMessage> {
    const plaintext = await crypto.decrypt(key, record.ciphertext, record.nonce);
    const content = JSON.parse(plaintext) as MessageContent;
    const blocks: DisplayBlock[] = content.blocks.map((block) =>
      block.type === "text"
        ? { kind: "text", text: block.text }
        : {
            kind: "image",
            image: {
              attachment_id: block.attachment_id,
              key_ciphertext: block.key_ciphertext,
              key_nonce: block.key_nonce,
            },
          }
    );
    return {
      message_id: record.message_id,
      conversation_id: record.conversation_id,
      mine: record.sender_device_id === deviceId,
      blocks,
      createdAt: record.created_at,
    };
  }

  async function hydrateBlock(key: string, block: DisplayBlock): Promise<DisplayBlock> {
    if (block.kind !== "image" || !block.image || block.imageUrl) return block;
    const attachmentKey = await crypto.decrypt(
      key,
      block.image.key_ciphertext,
      block.image.key_nonce
    );
    const attachment = await api.downloadAttachment(block.image.attachment_id);
    const bytes = await crypto.decryptAttachment(
      attachment.ciphertext,
      attachmentKey,
      attachment.nonce
    );
    const blob = new Blob([new Uint8Array(bytes)], { type: attachment.mime_type });
    return { ...block, imageUrl: URL.createObjectURL(blob) };
  }

  async function hydrateAll(key: string, list: DisplayMessage[]): Promise<DisplayMessage[]> {
    return Promise.all(
      list.map(async (message) => ({
        ...message,
        blocks: await Promise.all(message.blocks.map((block) => hydrateBlock(key, block))),
      }))
    );
  }

  function cacheable(message: DisplayMessage): DisplayMessage {
    return {
      ...message,
      blocks: message.blocks.map(({ imageUrl: _imageUrl, ...block }) => block),
    };
  }

  async function selectConversation(conv: ConversationInfo) {
    const sequence = ++selectionSequence.current;
    setActiveConv(conv);
    setMessages([]);
    setSelectingId(conv.conversation_id);
    setError("");

    try {
      const key = await ensureSessionKey(conv);
      const cached = (await storage.loadMessages(conv.conversation_id)) as DisplayMessage[];
      if (selectionSequence.current !== sequence) return;
      if (cached.length) setMessages(await hydrateAll(key, cached));

      const result = await api.listMessages(conv.conversation_id);
      const decrypted: DisplayMessage[] = [];
      for (const record of result.messages) {
        decrypted.push(await decryptRecord(key, record));
      }
      await storage.saveMessages(conv.conversation_id, decrypted.map(cacheable));
      if (selectionSequence.current !== sequence) return;

      setMessages(await hydrateAll(key, decrypted));
      setSummaries((current) => ({
        ...current,
        [conv.conversation_id]: summarizeConversation(decrypted, conv.created_at),
      }));
    } catch (selectError) {
      if (selectionSequence.current === sequence) setError(readableError(selectError));
    } finally {
      if (selectionSequence.current === sequence) setSelectingId(null);
    }
  }

  async function autoCreateConversation(user: UserInfo): Promise<ConversationInfo> {
    const devices = await api.getDevices(user.user_id);
    const peerPublicKey = devices.devices[0]?.public_identity;
    if (!peerPublicKey) throw new Error("对方尚未注册设备");

    const init = await crypto.initiateSession(peerPublicKey);
    const conversation = await api.createConversation({
      peer_user_id: user.user_id,
      ephemeral_pub: init.ephemeral_pub,
    });
    sessionKeys.current.set(conversation.conversation_id, init.session_key);
    await storage.saveSessionKey(conversation.conversation_id, init.session_key);
    return conversation;
  }

  async function openUser(user: UserInfo) {
    setOpeningUserId(user.user_id);
    setError("");
    try {
      let conversation = conversations.find((item) => item.peer_user_id === user.user_id);
      if (!conversation) {
        conversation = await autoCreateConversation(user);
        setConversations((current) => [
          conversation!,
          ...current.filter((item) => item.conversation_id !== conversation!.conversation_id),
        ]);
        setSummaries((current) => ({
          ...current,
          [conversation!.conversation_id]: summarizeConversation([], conversation!.created_at),
        }));
      }

      setSearchOpen(false);
      setSearch("");
      await selectConversation(conversation);
    } catch (openError) {
      setError(readableError(openError));
    } finally {
      setOpeningUserId(null);
    }
  }

  async function handleSendBlocks(blocks: ComposerBlock[]) {
    if (!activeConv) return;
    try {
      const key = await ensureSessionKey(activeConv);
      const contentBlocks: Block[] = [];
      const displayBlocks: DisplayBlock[] = [];

      for (const block of blocks) {
        if (block.type === "text") {
          contentBlocks.push({ type: "text", text: block.text });
          displayBlocks.push({ kind: "text", text: block.text });
          continue;
        }

        const encryptedAttachment = await crypto.encryptAttachment(block.bytes);
        const attachment = await api.uploadAttachment({
          ciphertext: encryptedAttachment.ciphertext,
          nonce: encryptedAttachment.nonce,
          mime_type: block.mimeType,
          size: block.bytes.length,
        });
        const encryptedKey = await crypto.encrypt(key, encryptedAttachment.key);
        const reference = {
          attachment_id: attachment.attachment_id,
          key_ciphertext: encryptedKey.ciphertext,
          key_nonce: encryptedKey.nonce,
        };
        contentBlocks.push({ type: "image", ...reference });
        const blob = new Blob([block.bytes], { type: block.mimeType });
        displayBlocks.push({ kind: "image", image: reference, imageUrl: URL.createObjectURL(blob) });
      }

      const content: MessageContent = { blocks: contentBlocks };
      const encrypted = await crypto.encrypt(key, JSON.stringify(content));
      const record = await api.sendMessage(activeConv.conversation_id, encrypted.ciphertext, encrypted.nonce);
      const display: DisplayMessage = {
        message_id: record.message_id,
        conversation_id: record.conversation_id,
        mine: true,
        blocks: displayBlocks,
        createdAt: record.created_at,
      };

      const nextMessages = [...messages, display];
      setMessages(nextMessages);
      void storage.saveMessages(activeConv.conversation_id, nextMessages.map(cacheable));
      setSummaries((summaryState) => ({
        ...summaryState,
        [activeConv.conversation_id]: summarizeConversation(nextMessages, activeConv.created_at),
      }));
    } catch (sendError) {
      setError(readableError(sendError));
      throw sendError;
    }
  }

  async function saveProfile(update: ProfileUpdate, nextProfile: ProfileInfo) {
    setProfileBusy(true);
    setProfileError("");
    try {
      await api.updateProfile(update);
      setProfile(nextProfile);
      setProfileOpen(false);
    } catch (saveError) {
      setProfileError(readableError(saveError));
    } finally {
      setProfileBusy(false);
    }
  }

  async function logout() {
    setToken(null);
    await crypto.clearToken().catch(() => undefined);
    await onLogout();
  }

  function openSearch() {
    setSearchOpen(true);
    requestAnimationFrame(() => searchInputRef.current?.focus());
  }

  return (
    <div className="chat-shell">
      <aside className="app-rail" aria-label="主导航">
        <button
          className="rail-avatar"
          type="button"
          title="个人资料"
          aria-label="打开个人资料"
          onClick={() => {
            setProfileError("");
            setProfileOpen(true);
          }}
        >
          <img src={profile?.avatar || DEFAULT_AVATAR} alt="我的头像" />
          <span className="online-dot" />
        </button>
        <button className="rail-item active" type="button" aria-current="page" title="消息">
          <MessageCircle size={21} strokeWidth={1.8} />
          <span>消息</span>
        </button>
        <button className="rail-item" type="button" title="个人资料" onClick={() => setProfileOpen(true)}>
          <UserRound size={20} strokeWidth={1.8} />
          <span>我的</span>
        </button>
        {profile?.username === "wangxin" && (
          <button className="rail-item" type="button" title="管理设置" onClick={() => setAdminSettingsOpen(true)}>
            <Settings2 size={20} strokeWidth={1.8} />
            <span>设置</span>
          </button>
        )}
        <div className="rail-spacer" />
        <div className="rail-security" title="消息采用端到端加密" role="status">
          <ShieldCheck size={19} strokeWidth={1.8} />
          <span>加密</span>
        </div>
        <button className="rail-item logout" type="button" title="退出登录" onClick={() => void logout()}>
          <LogOut size={19} strokeWidth={1.8} />
          <span>退出</span>
        </button>
      </aside>

      <aside className="conversation-panel">
        <div className="conversation-heading">
          <div>
              <span className="eyebrow">技术交流</span>
            <h1>消息</h1>
          </div>
          <button className="icon-button add-chat" type="button" title="发起会话" aria-label="发起会话" onClick={openSearch}>
            <Plus size={20} />
          </button>
        </div>

        <div className="search-area" ref={searchAreaRef}>
          <Search className="search-icon" size={16} />
          <input
            ref={searchInputRef}
            value={search}
            onFocus={() => setSearchOpen(true)}
            onChange={(event) => {
              setSearch(event.target.value);
              setSearchOpen(true);
            }}
            placeholder="搜索联系人"
            aria-label="搜索联系人"
          />
          {search && (
            <button className="clear-search" type="button" aria-label="清除搜索" onClick={() => setSearch("")}>
              <X size={14} />
            </button>
          )}

          {searchOpen && (
            <div className="search-popover" role="dialog" aria-label="联系人搜索结果">
              <div className="search-popover-title">
                <span>{search.trim() ? "搜索结果" : "全部联系人"}</span>
                <small>{filteredUsers.length} 位</small>
              </div>
              <div className="search-results">
                {filteredUsers.map((user) => {
                  const exists = conversations.some((conversation) => conversation.peer_user_id === user.user_id);
                  return (
                    <button key={user.user_id} type="button" onClick={() => void openUser(user)}>
                      <img src={user.avatar || DEFAULT_AVATAR} alt="" />
                      <span className="search-result-copy">
                        <strong>{user.username}</strong>
                        <small>{user.bio || (exists ? "已有会话" : "发起加密会话")}</small>
                      </span>
                      {openingUserId === user.user_id ? (
                        <LoaderCircle className="spin" size={17} />
                      ) : (
                        <span className="result-action">{exists ? "打开" : "会话"}</span>
                      )}
                    </button>
                  );
                })}
                {!filteredUsers.length && (
                  <div className="search-empty">
                    <Search size={23} />
                    <span>没有找到匹配的联系人</span>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="conversation-list" aria-label="会话列表">
          {loading ? (
            <div className="panel-loading"><LoaderCircle className="spin" size={20} /> 正在载入会话</div>
          ) : orderedConversations.length ? (
            orderedConversations.map((conversation) => {
              const summary = summaries[conversation.conversation_id];
              const user = users.find((item) => item.user_id === conversation.peer_user_id);
              const active = activeConv?.conversation_id === conversation.conversation_id;
              return (
                <button
                  key={conversation.conversation_id}
                  className={`conversation-item ${active ? "active" : ""}`}
                  type="button"
                  onClick={() => void selectConversation(conversation)}
                >
                  <img src={user?.avatar || conversation.peer_avatar || DEFAULT_AVATAR} alt="" />
                  <span className="conversation-copy">
                    <span className="conversation-name-row">
                      <strong>{conversation.peer_username}</strong>
                      <time>{formatConversationTime(summary?.timestamp || conversation.created_at)}</time>
                    </span>
                    <span className="conversation-preview">
                      {selectingId === conversation.conversation_id ? "正在载入消息…" : summary?.preview || "暂无消息"}
                    </span>
                  </span>
                </button>
              );
            })
          ) : (
            <div className="conversation-empty">
              <span className="empty-orb"><MessageCircle size={24} /></span>
              <strong>还没有会话</strong>
              <p>查找联系人，开始一段加密沟通。</p>
              <button type="button" onClick={openSearch}><Plus size={16} /> 发起会话</button>
            </div>
          )}
        </div>
      </aside>

      <main className="chat-main">
        {activeConv ? (
          <>
            <header className="chat-header">
              <button className="peer-heading" type="button" onClick={() => setPeerProfileOpen(true)}>
                <img src={peerProfile?.avatar || activeConv.peer_avatar || DEFAULT_AVATAR} alt="" />
                <span>
                  <strong>{activeConv.peer_username}</strong>
                  <small>{peerProfile?.bio || "端到端加密会话"}</small>
                </span>
              </button>
              <div className="encrypted-badge"><LockKeyhole size={13} /> 端到端加密</div>
            </header>

            <div className="messages" ref={messagesRef} aria-live="polite">
              {selectingId === activeConv.conversation_id && !messages.length ? (
                <div className="messages-loading"><LoaderCircle className="spin" size={20} /> 正在解密消息</div>
              ) : messages.length ? (
                messages.map((message) => (
                  <div key={message.message_id} className={`message-row ${message.mine ? "mine" : "peer"}`}>
                    {!message.mine && (
                      <button className="message-avatar" type="button" onClick={() => setPeerProfileOpen(true)}>
                        <img src={peerProfile?.avatar || activeConv.peer_avatar || DEFAULT_AVATAR} alt="" />
                      </button>
                    )}
                    <div className="message-stack">
                      <div className="message-bubble">
                        {message.blocks.map((block, index) =>
                          block.kind === "text" ? (
                            <p key={index}>{block.text}</p>
                          ) : block.imageUrl ? (
                            <img
                              key={index}
                              className="message-image"
                              src={block.imageUrl}
                              alt="聊天图片"
                              onClick={() => setLightbox(block.imageUrl || null)}
                            />
                          ) : null
                        )}
                      </div>
                      <time>{formatMessageTime(message.createdAt)}</time>
                    </div>
                    {message.mine && (
                      <button className="message-avatar" type="button" onClick={() => setProfileOpen(true)}>
                        <img src={profile?.avatar || DEFAULT_AVATAR} alt="" />
                      </button>
                    )}
                  </div>
                ))
              ) : (
                <div className="messages-empty">
                  <span><LockKeyhole size={22} /></span>
                  <strong>这是一段加密会话</strong>
                  <p>发送第一条消息，只有你和对方可以查看内容。</p>
                </div>
              )}
            </div>
            <Composer onSend={handleSendBlocks} />
          </>
        ) : (
          <div className="chat-empty-state">
            <div className="empty-illustration"><MessageCircle size={38} strokeWidth={1.5} /></div>
            <h2>选择一段会话</h2>
            <p>从左侧打开已有消息，或搜索联系人开始沟通。</p>
            <div><LockKeyhole size={14} /> 消息始终端到端加密</div>
          </div>
        )}

        {error && (
          <div className="error-toast" role="alert">
            <AlertCircle size={17} />
            <span>{error}</span>
            <button type="button" aria-label="关闭错误提示" onClick={() => setError("")}><X size={15} /></button>
          </div>
        )}
      </main>

      {profileOpen && profile && (
        <ProfileDialog
          mode="edit"
          profile={profile}
          busy={profileBusy}
          error={profileError}
          onClose={() => !profileBusy && setProfileOpen(false)}
          onSave={saveProfile}
        />
      )}
      {peerProfileOpen && peerProfile && (
        <ProfileDialog mode="view" profile={peerProfile} onClose={() => setPeerProfileOpen(false)} />
      )}
      {adminSettingsOpen && profile?.username === "wangxin" && (
        <AdminSettingsDialog onClose={() => setAdminSettingsOpen(false)} />
      )}
      {lightbox && <Lightbox src={lightbox} onClose={() => setLightbox(null)} />}
    </div>
  );
}

async function loadConversationSummaries(conversations: ConversationInfo[]) {
  const entries = await Promise.all(
    conversations.map(async (conversation) => {
      try {
        const cached = (await storage.loadMessages(conversation.conversation_id)) as DisplayMessage[];
        return [conversation.conversation_id, summarizeConversation(cached, conversation.created_at)] as const;
      } catch {
        return [conversation.conversation_id, summarizeConversation([], conversation.created_at)] as const;
      }
    })
  );
  return Object.fromEntries(entries) as Record<number, ConversationSummary>;
}

function summarizeConversation(messages: DisplayMessage[], fallbackTimestamp: string): ConversationSummary {
  const lastMessage = messages[messages.length - 1];
  const timestamp = lastMessage?.createdAt || fallbackTimestamp;
  return {
    preview: lastMessage ? messagePreview(lastMessage) : "暂无消息",
    timestamp,
    sortTime: serverDate(timestamp)?.getTime() || 0,
  };
}

function messagePreview(message: DisplayMessage) {
  const preview = message.blocks
    .map((block) => (block.kind === "text" ? block.text?.trim() : "[图片]"))
    .filter(Boolean)
    .join(" ")
    .replace(/\s+/g, " ");
  return preview || "暂无消息";
}

function sortConversations(
  conversations: ConversationInfo[],
  summaries: Record<number, ConversationSummary>
) {
  return [...conversations].sort((left, right) => {
    const rightTime = summaries[right.conversation_id]?.sortTime ?? serverDate(right.created_at)?.getTime() ?? 0;
    const leftTime = summaries[left.conversation_id]?.sortTime ?? serverDate(left.created_at)?.getTime() ?? 0;
    return rightTime - leftTime;
  });
}

function serverDate(value?: string) {
  if (!value) return null;
  const hasTimezone = value.includes("T") || /(?:Z|[+-]\d{2}:\d{2})$/.test(value);
  const normalized = hasTimezone ? value : `${value.replace(" ", "T")}Z`;
  const date = new Date(normalized);
  return Number.isNaN(date.getTime()) ? null : date;
}

function formatConversationTime(value?: string) {
  const date = serverDate(value);
  if (!date) return "";
  const now = new Date();
  if (date.toDateString() === now.toDateString()) {
    return new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false }).format(date);
  }
  if (date.getFullYear() === now.getFullYear()) {
    return new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit" }).format(date);
  }
  return new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" }).format(date);
}

function formatMessageTime(value?: string) {
  const date = serverDate(value);
  if (!date) return "";
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(date);
}

async function loadProfileWithLegacyFallback(): Promise<ProfileInfo> {
  try {
    return await api.getProfile();
  } catch (error) {
    if (!String(error).includes("HTTP 404")) throw error;
    return {
      user_id: Number(localStorage.getItem("technology-communication.user_id") || 0),
      username: localStorage.getItem("technology-communication.username") || "我",
      avatar: null,
      bio: "",
    };
  }
}

function readableError(error: unknown) {
  return String(error).replace(/^Error:\s*/, "") || "操作失败，请稍后重试";
}
