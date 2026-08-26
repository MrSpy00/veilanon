<script lang="ts">
  /**
   * veilanon AppLogo — tek kaynak marka logo component'i.
   * Kaynak: static/brand/veilanon-logo.svg (raster-embedded, lossless).
   * Her yerde (onboarding, splash, sidebar, hakkında, ayarlar) aynı logo
   * kullanılır; buradan değiştirmek tüm uygulamaya yayılır.
   */
  let {
    size = 56,
    radius = 16,
    alt = 'veilanon',
    title,
    class: className = '',
  }: {
    size?: number;
    /** Köşe yuvarlaklığı (px). Varsayılan yumuşak; keskin istemler 0 verir. */
    radius?: number;
    alt?: string;
    title?: string;
    class?: string;
  } = $props();

  let computedRadius = $derived(radius ?? Math.max(8, Math.round(size * 0.32)));
</script>

<img
  src="/brand/veilanon-logo.svg"
  {alt}
  {title}
  class="veil-app-logo {className}"
  style:width="{size}px"
  style:height="{size}px"
  style:border-radius="{computedRadius}px"
  draggable="false"
/>

<style>
  .veil-app-logo {
    display: inline-block;
    object-fit: cover;
    flex-shrink: 0;
    overflow: hidden;
    user-select: none;
    -webkit-user-drag: none;
    transition: transform var(--t-fast), box-shadow var(--t-fast), border-radius var(--t-fast);
  }
</style>
