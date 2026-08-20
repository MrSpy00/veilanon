/**
 * veilanon — Channel & DM Message Drafts Store
 *
 * Persists un-sent message content, pending attachments and reply targets
 * per-channel/DM across navigation until the application session closes.
 */
import { writable, get } from 'svelte/store';

export interface ChannelDraft {
  content: string;
  replyTo?: {
    channelId: string;
    messageId: string;
    author: string;
    content: string;
  } | null;
  files?: Array<{
    name: string;
    fileId: string;
    r2Key: string;
    sizeBytes: number;
    mimeTypeHint: string | null;
  }>;
  updatedAt: number;
}

interface DraftState {
  drafts: Record<string, ChannelDraft>;
}

const DRAFT_STORAGE_KEY = 'veilanon_session_drafts';

function loadSessionDrafts(): Record<string, ChannelDraft> {
  if (typeof window === 'undefined') return {};
  try {
    const raw = sessionStorage.getItem(DRAFT_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === 'object') return parsed;
    }
  } catch {
    // Quota or security sandbox
  }
  return {};
}

function saveSessionDrafts(drafts: Record<string, ChannelDraft>) {
  if (typeof window === 'undefined') return;
  try {
    sessionStorage.setItem(DRAFT_STORAGE_KEY, JSON.stringify(drafts));
  } catch {
    // Ignore storage quota
  }
}

function createDraftStore() {
  const { subscribe, update } = writable<DraftState>({
    drafts: loadSessionDrafts(),
  });

  return {
    subscribe,

    getDraft(channelId: string): ChannelDraft | undefined {
      if (!channelId) return undefined;
      return get({ subscribe }).drafts[channelId];
    },

    setDraft(
      channelId: string,
      content: string,
      replyTo?: ChannelDraft['replyTo'],
      files?: ChannelDraft['files']
    ) {
      if (!channelId) return;
      const hasContent = content.trim().length > 0;
      const hasFiles = (files && files.length > 0);

      update(s => {
        const next = { ...s.drafts };
        if (!hasContent && !hasFiles) {
          delete next[channelId];
        } else {
          next[channelId] = {
            content,
            replyTo: replyTo ?? null,
            files: files ?? [],
            updatedAt: Date.now(),
          };
        }
        saveSessionDrafts(next);
        return { ...s, drafts: next };
      });
    },

    clearDraft(channelId: string) {
      if (!channelId) return;
      update(s => {
        if (!s.drafts[channelId]) return s;
        const next = { ...s.drafts };
        delete next[channelId];
        saveSessionDrafts(next);
        return { ...s, drafts: next };
      });
    },
  };
}

export const draftStore = createDraftStore();
