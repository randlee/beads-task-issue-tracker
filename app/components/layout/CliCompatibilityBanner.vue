<script setup lang="ts">
import { AlertTriangleIcon, XCircleIcon, XIcon, SettingsIcon } from 'lucide-vue-next'

const { status, isVisible, dismiss } = useCliCompatibility()
const { showSettingsDialog } = useAppMenu()

const isError = computed(() => status.value?.level === 'error')
</script>

<template>
  <div
    v-if="isVisible && status"
    role="alert"
    class="flex items-start gap-3 px-4 py-2 text-sm border-b"
    :class="isError
      ? 'bg-destructive/10 text-destructive border-destructive/30'
      : 'bg-amber-500/10 text-amber-700 dark:text-amber-300 border-amber-500/30'"
  >
    <component :is="isError ? XCircleIcon : AlertTriangleIcon" class="w-4 h-4 shrink-0 mt-0.5" />

    <div class="flex-1 min-w-0 space-y-1">
      <div class="font-medium">{{ status.title }}</div>
      <ul v-if="status.details.length" class="list-disc pl-4 space-y-0.5 text-xs opacity-90">
        <li v-for="(line, i) in status.details" :key="i">{{ line }}</li>
      </ul>
      <details v-if="status.searchedPaths.length" class="text-xs opacity-80">
        <summary class="cursor-pointer select-none">Searched directories</summary>
        <ul class="font-mono pl-4 pt-1 space-y-0.5 break-all">
          <li v-for="p in status.searchedPaths" :key="p">{{ p }}</li>
        </ul>
      </details>
    </div>

    <div class="flex items-center gap-1 shrink-0">
      <button
        class="inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
        title="Open Settings (⌘,)"
        @click="showSettingsDialog = true"
      >
        <SettingsIcon class="w-3.5 h-3.5" />
        Settings
      </button>
      <button
        v-if="status.dismissible"
        class="p-1 rounded-md hover:bg-black/5 dark:hover:bg-white/10 transition-colors"
        title="Dismiss until the CLI changes"
        aria-label="Dismiss"
        @click="dismiss"
      >
        <XIcon class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
</template>
