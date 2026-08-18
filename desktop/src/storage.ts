// Local at-rest encrypted storage. All values are encrypted with the Local
// Master Key (LMK, held in the OS keychain) via WebCrypto AES-256-GCM before
// being written to IndexedDB, so third-party tools cannot read plaintext.

import { crypto as tauriCrypto } from "./crypto";

const DB_NAME = "technology-communication";
const STORE_SESSIONS = "sessions";
const STORE_MESSAGES = "messages";

let lmkCache: Uint8Array<ArrayBuffer> | null = null;

function b64ToBytes(b64: string): Uint8Array<ArrayBuffer> {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

function bytesToB64(bytes: Uint8Array<ArrayBuffer>): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

async function getLmk(): Promise<Uint8Array<ArrayBuffer>> {
  if (lmkCache) return lmkCache;
  const b64 = await tauriCrypto.getOrCreateLmk();
  lmkCache = b64ToBytes(b64);
  return lmkCache;
}

async function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, 1);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_SESSIONS)) db.createObjectStore(STORE_SESSIONS);
      if (!db.objectStoreNames.contains(STORE_MESSAGES)) db.createObjectStore(STORE_MESSAGES);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function dbPut(store: string, key: string, value: string): Promise<void> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(store, "readwrite");
    tx.objectStore(store).put(value, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function dbGet(store: string, key: string): Promise<string | undefined> {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(store, "readonly");
    const req = tx.objectStore(store).get(key);
    req.onsuccess = () => resolve(req.result as string | undefined);
    req.onerror = () => reject(req.error);
  });
}

async function encrypt(value: string): Promise<{ ciphertext: string; nonce: string }> {
  const key = await getLmk();
  const cryptoKey = await crypto.subtle.importKey("raw", key, "AES-GCM", false, ["encrypt"]);
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const data = new TextEncoder().encode(value);
  const ct = await crypto.subtle.encrypt({ name: "AES-GCM", iv: nonce }, cryptoKey, data);
  return { ciphertext: bytesToB64(new Uint8Array(ct)), nonce: bytesToB64(nonce) };
}

async function decrypt(ciphertext: string, nonce: string): Promise<string> {
  const key = await getLmk();
  const cryptoKey = await crypto.subtle.importKey("raw", key, "AES-GCM", false, ["decrypt"]);
  const data = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: b64ToBytes(nonce) },
    cryptoKey,
    b64ToBytes(ciphertext)
  );
  return new TextDecoder().decode(data);
}

export const storage = {
  async saveSessionKey(conversationId: number, sessionKey: string): Promise<void> {
    const { ciphertext, nonce } = await encrypt(sessionKey);
    await dbPut(STORE_SESSIONS, String(conversationId), JSON.stringify({ ciphertext, nonce }));
  },

  async loadSessionKey(conversationId: number): Promise<string | null> {
    const raw = await dbGet(STORE_SESSIONS, String(conversationId));
    if (!raw) return null;
    const { ciphertext, nonce } = JSON.parse(raw);
    return decrypt(ciphertext, nonce);
  },

  async saveMessages(conversationId: number, messages: unknown[]): Promise<void> {
    const { ciphertext, nonce } = await encrypt(JSON.stringify(messages));
    await dbPut(STORE_MESSAGES, String(conversationId), JSON.stringify({ ciphertext, nonce }));
  },

  async loadMessages(conversationId: number): Promise<unknown[]> {
    const raw = await dbGet(STORE_MESSAGES, String(conversationId));
    if (!raw) return [];
    const { ciphertext, nonce } = JSON.parse(raw);
    return JSON.parse(await decrypt(ciphertext, nonce));
  },
};
