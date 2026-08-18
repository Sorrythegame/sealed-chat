// HTTP client for the Technology Communication relay server.

export interface AuthResponse {
  user_id: number;
  device_id: number;
  username: string;
  token: string;
}

export interface DeviceInfo {
  device_id: number;
  user_id: number;
  device_name: string;
  public_identity: string;
}

export interface UserInfo {
  user_id: number;
  username: string;
  avatar: string | null;
  bio: string;
}

export interface ProfileInfo {
  user_id: number;
  username: string;
  avatar: string | null;
  bio: string;
}

export interface ProfileUpdate {
  avatar?: string;
  bio?: string;
}

export interface InviteCreateResponse {
  code: string;
  expires_at: string;
}

export interface ConversationInfo {
  conversation_id: number;
  peer_user_id: number;
  peer_username: string;
  ephemeral_pub: string | null;
  peer_avatar: string | null;
  created_at: string;
}

export interface MessageRecord {
  message_id: number;
  conversation_id: number;
  sender_device_id: number;
  ciphertext: string;
  nonce: string;
  created_at: string;
}

const BASE_URL = "http://111.228.33.56:8080";
const baseUrl = () => BASE_URL;
const token = () => localStorage.getItem("technology-communication.token");

export function setToken(value: string | null) {
  if (value) localStorage.setItem("technology-communication.token", value);
  else localStorage.removeItem("technology-communication.token");
}

export function getToken() {
  return token();
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(options.headers as Record<string, string>),
  };
  const t = token();
  if (t) headers.Authorization = `Bearer ${t}`;
  const res = await fetch(`${baseUrl()}${path}`, { ...options, headers });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`HTTP ${res.status}: ${body}`);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const api = {
  register: (body: {
    username: string;
    password: string;
    invite_code: string;
    device_name: string;
    public_identity: string;
  }) =>
    request<AuthResponse>("/api/auth/register", { method: "POST", body: JSON.stringify(body) }),

  login: (body: { username: string; password: string }) =>
    request<AuthResponse>("/api/auth/login", { method: "POST", body: JSON.stringify(body) }),

  listUsers: () => request<{ users: UserInfo[] }>("/api/users"),

  getProfile: () => request<ProfileInfo>("/api/me"),

  updateProfile: (body: ProfileUpdate) =>
    request<void>("/api/me", { method: "PATCH", body: JSON.stringify(body) }),

  getUserByName: (username: string) =>
    request<{ user_id: number; username: string }>(`/api/users/by-name/${encodeURIComponent(username)}`),

  getDevices: (userId: number) =>
    request<{ devices: DeviceInfo[] }>(`/api/users/${userId}/devices`),

  createConversation: (body: { peer_user_id: number; ephemeral_pub: string }) =>
    request<ConversationInfo>("/api/conversations", { method: "POST", body: JSON.stringify(body) }),

  listConversations: () => request<{ conversations: ConversationInfo[] }>("/api/conversations"),

  sendMessage: (conversationId: number, ciphertext: string, nonce: string) =>
    request<MessageRecord>(`/api/conversations/${conversationId}/messages`, {
      method: "POST",
      body: JSON.stringify({ ciphertext, nonce }),
    }),

  listMessages: (conversationId: number, after = 0) =>
    request<{ messages: MessageRecord[] }>(`/api/conversations/${conversationId}/messages?after=${after}`),

  uploadAttachment: (body: { ciphertext: string; nonce: string; mime_type: string; size: number }) =>
    request<{ attachment_id: string }>("/api/attachments", {
      method: "POST",
      body: JSON.stringify(body),
    }),

  downloadAttachment: (id: string) =>
    request<{ ciphertext: string; nonce: string; mime_type: string; size: number }>(
      `/api/attachments/${encodeURIComponent(id)}`
    ),

  updateAvatar: (avatar: string) =>
    request<void>("/api/me/avatar", { method: "POST", body: JSON.stringify({ avatar }) }),

  createInvite: () =>
    request<InviteCreateResponse>("/api/invites", { method: "POST", body: "{}" }),
};
