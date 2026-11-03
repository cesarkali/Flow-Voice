<script lang="ts">
  // UI language picker: "auto" follows Windows; each language is listed by its own name.
  import { t } from "../i18n.svelte";
  import type { UiLanguagePref } from "../types";
  import Select from "./Select.svelte";

  let { value = $bindable("auto"), label }: { value: UiLanguagePref; label: string } = $props();

  const NAMES: { value: UiLanguagePref; label: string }[] = [
    { value: "pt", label: "Português" },
    { value: "en", label: "English" },
    { value: "es", label: "Español" },
    { value: "fr", label: "Français" },
    { value: "de", label: "Deutsch" },
    { value: "ja", label: "日本語" },
    { value: "zh", label: "简体中文" },
    { value: "ru", label: "Русский" },
    { value: "hi", label: "हिन्दी" },
    { value: "bn", label: "বাংলা" },
  ];
  const options = $derived([{ value: "auto" as UiLanguagePref, label: t("gen.ui_language_auto") }, ...NAMES]);
</script>

<Select bind:value {options} {label} />
