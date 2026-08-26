/**
 * veilanon — Discord bridge adapter.
 *
 * UYGULANAN YOL (webhook): kanal sahibi Discord'da kendi sunucusu için webhook
 * oluşturur; URL yalnızca kendi cihazında saklanır; gönderilen mesajlar
 * "[köprü]" etiketiyle yansıtılır ve Discord tarafında E2EE koruması YOKTUR.
 *
 * OAuth2 akışı (Discord uygulama kimlikleri gerektirir): token'lar hiçbir
 * veilanon sunucusunda saklanmaz; alışveriş tamamen kullanıcı cihazı ile
 * Discord arasında olur. Uygulama kimlikleri eklendiğinde bu dosyadaki
 * sözleşme üzerinden bağlanır.
 */

export type BridgeScope = 'none' | 'metadata' | 'messages';

export interface BridgeConsent {
  /** Which data surface the user consents to bridging. */
  scope: BridgeScope;
  /** Human-readable, Turkish consent confirmation text the user has seen. */
  consentText: string;
  /** Timestamp of consent (ms epoch) — set by the caller. */
  consentedAt?: number;
}

export interface BridgeStatus {
  active: boolean;
  scope: BridgeScope;
  message: string;
}

const BRIDGE_LABEL = 'bridged';

/** Label used on messages that arrived via the bridge (NOT E2EE). */
export function bridgedMessageLabel(): string {
  return BRIDGE_LABEL;
}

/**
 * Initiate the bridge OAuth2 flow.
 * REQUIRES an explicit consent object — the UI must show the consent dialog first and
 * pass the user-confirmed scope + text here. Throws if consent is missing or malformed.
 */
export async function initiateBridge(consent: BridgeConsent): Promise<BridgeStatus> {
  if (!consent || !consent.consentText || consent.consentText.trim().length === 0) {
    throw new Error('Köprü için açık onay gerekli.');
  }
  // Stub: Rust `discord_bridge_start(consent)` will open the system browser for OAuth2.
  // No token is returned to the frontend by design.
  return {
    active: true,
    scope: consent.scope,
    message:
      'Köprü aktif. Köprüden gelen mesajlar "bridged" olarak etiketlenir ve veilanon içinde uçtan uca şifreli DEĞİLDİR.',
  };
}

/** Revoke the bridge. Removes the local device scope; no server-side token exists to delete. */
export async function revokeBridge(): Promise<BridgeStatus> {
  // Stub: Rust `discord_bridge_revoke()` clears the local scope.
  return { active: false, scope: 'none', message: 'Köprü kaldırıldı.' };
}
