<script lang="ts">
  let {
    checked = false,
    disabled = false,
    label = '',
    id,
    onChange,
  }: {
    checked?: boolean;
    disabled?: boolean;
    label?: string;
    id?: string;
    onChange?: (checked: boolean) => void;
  } = $props();

  const inputId = $derived(id ?? `veil-toggle-${Math.random().toString(36).slice(2, 8)}`);
</script>

<label class="veil-toggle" for={inputId}>
  <input
    id={inputId}
    type="checkbox"
    checked={checked}
    {disabled}
    aria-label={label || undefined}
    onchange={(e) => onChange?.((e.currentTarget as HTMLInputElement).checked)}
  />
  <span class="veil-toggle-track" aria-hidden="true"></span>
</label>
