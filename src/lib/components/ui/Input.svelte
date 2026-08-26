<script lang="ts">
  let {
    label = '',
    id,
    value,
    type = 'text',
    placeholder = '',
    disabled = false,
    error = false,
    hint = '',
    maxlength,
    autocomplete = 'off',
    oninput,
  }: {
    label?: string;
    id?: string;
    value: string;
    type?: 'text' | 'password' | 'email' | 'number' | 'search';
    placeholder?: string;
    disabled?: boolean;
    error?: boolean;
    hint?: string;
    maxlength?: number;
    autocomplete?: string;
    oninput?: (value: string) => void;
  } = $props();

  const inputId = $derived(id ?? `veil-input-${Math.random().toString(36).slice(2, 8)}`);
</script>

<div class="veil-field">
  {#if label}
    <label class="veil-form-label" for={inputId}>{label}</label>
  {/if}
  <input
    {id}
    class="veil-input"
    class:error
    {type}
    value={value}
    {placeholder}
    {disabled}
    {maxlength}
    autocomplete={autocomplete as HTMLInputElement['autocomplete']}
    aria-label={label || undefined}
    aria-invalid={error || undefined}
    oninput={(e) => oninput?.((e.currentTarget as HTMLInputElement).value)}
  />
  {#if hint}
    <span class="veil-form-desc">{hint}</span>
  {/if}
</div>

<style>
  .veil-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
</style>
