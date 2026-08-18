// Thin wrappers over the Tauri Rust crypto commands.

import { invoke } from "@tauri-apps/api/core";

export interface SessionInit {
  ephemeral_pub: string;
  session_key: string;
}

export interface Cipher {
  ciphertext: string;
  nonce: string;
}

export interface AttachmentCipher {
  ciphertext: string;
  key: string;
  nonce: string;
}

export const crypto = {
  generateIdentity: () => invoke<{ public_identity: string }>("generate_identity"),

  initiateSession: (peerPublicIdentity: string) =>
    invoke<SessionInit>("initiate_session", { peerPublicIdentity }),

  completeSession: (peerPublicIdentity: string, peerEphemeralPub: string) =>
    invoke<string>("complete_session", { peerPublicIdentity, peerEphemeralPub }),

  encrypt: (sessionKey: string, plaintext: string) =>
    invoke<Cipher>("encrypt_message", { sessionKey, plaintext }),

  decrypt: (sessionKey: string, ciphertext: string, nonce: string) =>
    invoke<string>("decrypt_message", { sessionKey, ciphertext, nonce }),

  getOrCreateLmk: () => invoke<string>("get_or_create_lmk"),

  encryptAttachment: (data: number[] | Uint8Array) =>
    invoke<AttachmentCipher>("encrypt_attachment", { data }),

  decryptAttachment: (ciphertext: string, key: string, nonce: string) =>
    invoke<number[]>("decrypt_attachment", { ciphertext, key, nonce }),

  screenshot: () => invoke<number[]>("screenshot"),

  saveToken: (token: string) => invoke<void>("save_token", { token }),
  getToken: () => invoke<string | null>("get_token"),
  clearToken: () => invoke<void>("clear_token"),
};
