<script setup lang="ts">
import type { Issue, IssueStatus } from '~/types/issue'
import type { HistoryEntry } from '~/utils/history-helpers'
import { truncateTitle, shortIssueId } from '~/utils/history-helpers'
import { ArrowLeftIcon, ArrowRightIcon, HistoryIcon } from 'lucide-vue-next'
import TypeBadge from '~/components/issues/TypeBadge.vue'
import StatusBadge from '~/components/issues/StatusBadge.vue'
import PriorityBadge from '~/components/issues/PriorityBadge.vue'
import { Button } from '~/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '~/components/ui/dropdown-menu'

withDefaults(defineProps<{
  selectedIssue: Issue
  isPinned?: boolean
  canGoBack?: boolean
  canGoForward?: boolean
  historyEntries?: HistoryEntry[]
}>(), {
  isPinned: false,
  canGoBack: false,
  canGoForward: false,
  historyEntries: () => [],
})

defineEmits<{
  edit: []
  reopen: []
  close: []
  delete: []
  'toggle-pin': []
  back: []
  forward: []
  'select-history': [id: string]
}>()
</script>

<template>
  <div class="p-4 pb-0 space-y-3 border-b border-border">
    <!-- Badges row -->
    <div class="flex items-center gap-1.5 flex-wrap">
      <CopyableId :value="selectedIssue.id" :display-value="selectedIssue.id.includes('-') ? selectedIssue.id.slice(selectedIssue.id.lastIndexOf('-') + 1) : selectedIssue.id" />
      <TypeBadge :type="selectedIssue.type" size="sm" />
      <StatusBadge :status="selectedIssue.status" size="sm" />
      <PriorityBadge :priority="selectedIssue.priority" size="sm" />
    </div>

    <!-- Title -->
    <h3 class="text-sm font-semibold line-clamp-2">{{ selectedIssue.title }}</h3>

    <!-- Action buttons -->
    <div class="flex items-center justify-between pb-3">
      <div class="flex items-center gap-1">
        <!-- Navigation history: back / forward / recently viewed -->
        <Button
          variant="ghost"
          size="sm"
          class="h-7 w-7 p-0"
          :disabled="!canGoBack"
          title="Back (Alt+Left)"
          aria-label="Back to previous issue"
          @click="$emit('back')"
        >
          <ArrowLeftIcon class="w-3.5 h-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          class="h-7 w-7 p-0"
          :disabled="!canGoForward"
          title="Forward (Alt+Right)"
          aria-label="Forward to next issue"
          @click="$emit('forward')"
        >
          <ArrowRightIcon class="w-3.5 h-3.5" />
        </Button>
        <DropdownMenu :modal="false">
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost"
              size="sm"
              class="h-7 w-7 p-0"
              :disabled="historyEntries.length === 0"
              title="Recently viewed"
              aria-label="Recently viewed issues"
            >
              <HistoryIcon class="w-3.5 h-3.5" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" class="w-80 max-h-96 overflow-y-auto">
            <DropdownMenuLabel class="text-xs text-muted-foreground">Recently viewed</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              v-for="entry in historyEntries"
              :key="entry.id"
              class="gap-2 text-xs"
              :class="entry.id === selectedIssue.id ? 'font-semibold' : ''"
              @select="$emit('select-history', entry.id)"
            >
              <span class="font-mono text-muted-foreground shrink-0">{{ shortIssueId(entry.id) }}</span>
              <span class="flex-1 truncate" :title="entry.title">{{ truncateTitle(entry.title, 48) }}</span>
              <StatusBadge :status="entry.status as IssueStatus" size="sm" />
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <span class="w-px h-5 bg-border mx-0.5" aria-hidden="true" />

        <!-- Pin/unpin toggle -->
        <Button
          variant="outline"
          size="sm"
          class="h-7 text-xs px-2"
          :class="isPinned ? 'text-amber-500 border-amber-500/50 hover:text-amber-600' : ''"
          @click="$emit('toggle-pin')"
        >
          <svg class="w-3 h-3 mr-1" viewBox="0 0 24 24" :fill="isPinned ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 4v6l-2 4h10l-2-4V4" /><line x1="12" y1="16" x2="12" y2="21" /><line x1="8" y1="4" x2="16" y2="4" />
          </svg>
          {{ isPinned ? 'Unpin' : 'Pin' }}
        </Button>
        <!-- Edit button: only when not closed -->
        <Button v-if="selectedIssue.status !== 'closed'" size="sm" class="h-7 text-xs px-2" @click="$emit('edit')">
          <svg class="w-3 h-3 mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
          </svg>
          Edit
        </Button>
        <!-- Reopen button: only when closed -->
        <Button
          v-if="selectedIssue.status === 'closed'"
          size="sm"
          class="h-7 text-xs px-2"
          @click="$emit('reopen')"
        >
          <svg class="w-3 h-3 mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
            <path d="M3 3v5h5" />
          </svg>
          Reopen
        </Button>
        <!-- Close button: only when not closed -->
        <Button
          v-if="selectedIssue.status !== 'closed'"
          variant="outline"
          size="sm"
          class="h-7 text-xs px-2"
          @click="$emit('close')"
        >
          <svg class="w-3 h-3 mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          Close
        </Button>
      </div>
      <Button
        variant="outline"
        size="sm"
        class="h-7 text-xs px-2 text-destructive hover:bg-destructive hover:text-destructive-foreground"
        @click="$emit('delete')"
      >
        <svg class="w-3 h-3 mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6" />
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          <line x1="10" y1="11" x2="10" y2="17" />
          <line x1="14" y1="11" x2="14" y2="17" />
        </svg>
        Delete
      </Button>
    </div>
  </div>
</template>
