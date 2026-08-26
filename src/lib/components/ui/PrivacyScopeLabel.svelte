<script lang="ts">
  /**
   * Honest privacy-scope label.
   * `content`  → message content is E2EE (never readable by the server).
   * `metadata` → only metadata (who/when) is visible; content stays local/E2EE.
   * `none`     → neither content nor metadata is protected.
   * `partial`  → mixed: some messages protected, some not (e.g. bridge).
   */

  export type PrivacyScope = 'content' | 'metadata' | 'none' | 'partial';

  import Icon from './Icon.svelte';

  let {
    scope = 'content' as PrivacyScope,
    compact = false,
    title = '',
  }: {
    scope?: PrivacyScope;
    compact?: boolean;
    title?: string;
  } = $props();

  const meta = $derived(
    scope === 'content'
      ? { cls: 'e2ee', label: compact ? 'E2EE' : 'İçerik uçtan uca şifreli', tip: 'Mesaj içeriği sunucu tarafından okunamaz.', icon: 'lock' as const }
      : scope === 'metadata'
        ? { cls: 'partial', label: compact ? 'Meta' : 'Yalnızca meta veri', tip: 'İçerik şifreli; kimin/ne zaman yazdığı görülebilir.', icon: 'shield' as const }
        : scope === 'partial'
          ? { cls: 'partial', label: compact ? 'Karma' : 'Kısmen şifreli', tip: 'Bazı mesajlar (ör. bridged) uçtan uca şifreli değildir.', icon: 'unlock' as const }
          : { cls: 'none', label: compact ? 'Açık' : 'Şifrelenmemiş', tip: 'Bu kanalda uçtan uca şifreleme yoktur.', icon: 'warning' as const }
  );
</script>

<span
  class="veil-privacy-scope {meta.cls}"
  title={title || meta.tip}
>
  <Icon name={meta.icon} size={12} />
  {meta.label}
</span>
