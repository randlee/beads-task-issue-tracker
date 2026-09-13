<script setup lang="ts">
import type { IssueMetadata } from '~/types/issue'
import { Badge } from '~/components/ui/badge'
import {
  partitionMetadata,
  formatEffort,
  formatGenericValue,
  parseWikiLink,
  toStringList,
  badgeTone,
  severityTone,
  formatDateOnly,
  type ResolvedField,
} from '~/utils/custom-fields'
import { formatMetadataJson } from '~/utils/metadata'

const props = defineProps<{
  metadata: IssueMetadata
}>()

const parts = computed(() => partitionMetadata(props.metadata))
const showRaw = ref(false)

/** Only show group headings when a namespaced group is present or there are several groups. */
const showHeadings = computed(() =>
  parts.value.groups.length > 1 || parts.value.groups.some(g => g.namespaced),
)

const numberText = (field: ResolvedField): string => {
  const text = formatGenericValue(field.value)
  return field.def.unit ? `${text} ${field.def.unit}` : text
}
</script>

<template>
  <div class="space-y-2">
    <!-- Effort summary -->
    <div v-if="parts.effort" class="rounded border border-border/60 bg-muted/30 p-2 space-y-1.5">
      <div class="grid grid-cols-4 gap-2 text-center">
        <div v-for="col in [
          { label: 'Original', value: parts.effort.original },
          { label: 'Remaining', value: parts.effort.remaining },
          { label: 'Completed', value: parts.effort.completed },
        ]" :key="col.label">
          <div class="text-[10px] uppercase tracking-wide text-muted-foreground">{{ col.label }}</div>
          <div class="text-xs font-mono">{{ formatEffort(col.value, parts.effort.unit) }}</div>
        </div>
        <div>
          <div class="text-[10px] uppercase tracking-wide text-muted-foreground">% Done</div>
          <div class="text-xs font-mono">{{ parts.effort.percent === null ? '—' : `${parts.effort.percent}%` }}</div>
        </div>
      </div>
      <div
        v-if="parts.effort.percent !== null"
        class="h-1.5 w-full rounded-full bg-muted overflow-hidden"
        role="progressbar"
        :aria-valuenow="parts.effort.percent"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <div class="h-full bg-primary transition-[width]" :style="{ width: `${parts.effort.percent}%` }" />
      </div>
    </div>

    <!-- Schedule summary -->
    <div v-if="parts.schedule" class="flex items-center gap-2 text-xs flex-wrap">
      <span class="text-[10px] uppercase tracking-wide text-muted-foreground">Schedule</span>
      <span class="font-mono">{{ parts.schedule.start ?? '—' }}</span>
      <span class="text-muted-foreground">→</span>
      <span class="font-mono">{{ parts.schedule.end ?? '—' }}</span>
      <span v-if="parts.schedule.durationDays !== null" class="text-muted-foreground">
        ({{ parts.schedule.durationDays }} {{ parts.schedule.durationDays === 1 ? 'day' : 'days' }})
      </span>
      <span v-if="parts.schedule.inverted" class="text-amber-600 dark:text-amber-400" title="End date is before start date">
        ends before it starts
      </span>
    </div>

    <!-- Grouped fields -->
    <div v-for="group in parts.groups" :key="group.id" class="space-y-1">
      <h5 v-if="showHeadings" class="text-[10px] font-medium text-sky-400 uppercase tracking-wide">
        {{ group.label }}
      </h5>
      <dl class="grid grid-cols-[minmax(6rem,auto)_1fr] gap-x-3 gap-y-1 text-xs">
        <template v-for="field in group.fields" :key="field.key">
          <dt class="text-muted-foreground truncate" :title="field.key">{{ field.def.label }}</dt>
          <dd class="min-w-0 break-words">
            <!-- badge -->
            <Badge
              v-if="field.def.kind === 'badge' && typeof field.value === 'string'"
              variant="outline"
              class="text-[10px] px-1.5 py-0"
              :class="badgeTone(field.def, field.value)"
            >{{ field.value }}</Badge>

            <!-- severity -->
            <Badge
              v-else-if="field.def.kind === 'severity' && typeof field.value === 'string'"
              variant="outline"
              class="text-[10px] px-1.5 py-0"
              :class="severityTone(field.value)"
            >{{ field.value }}</Badge>

            <!-- wiki links -->
            <div v-else-if="field.def.kind === 'wikilinks' && toStringList(field.value).length" class="flex flex-wrap gap-1">
              <CopyableId
                v-for="link in toStringList(field.value).map(parseWikiLink)"
                :key="link.raw"
                :value="link.raw"
                :display-value="link.alias ?? link.target"
              />
            </div>

            <!-- copyable id -->
            <CopyableId
              v-else-if="field.def.kind === 'copyable-id' && typeof field.value === 'string' && field.value"
              :value="field.value"
            />

            <!-- date -->
            <span v-else-if="field.def.kind === 'date'" class="font-mono">{{ formatDateOnly(field.value) }}</span>

            <!-- number / effort -->
            <span v-else-if="field.def.kind === 'number' || field.def.kind === 'effort'" class="font-mono">{{ numberText(field) }}</span>

            <!-- text / generic (also the fallback when a typed widget gets an unexpected value) -->
            <span v-else :class="{ 'font-mono': typeof field.value !== 'string' }">{{ formatGenericValue(field.value) }}</span>
          </dd>
        </template>
      </dl>
    </div>

    <!-- Raw JSON -->
    <div>
      <button
        class="text-[10px] text-muted-foreground hover:text-foreground transition-colors"
        @click="showRaw = !showRaw"
      >
        {{ showRaw ? 'Hide raw JSON' : 'Show raw JSON' }}
      </button>
      <pre v-if="showRaw" class="mt-1 text-xs bg-muted/50 rounded p-2 overflow-x-auto whitespace-pre-wrap break-words">{{ formatMetadataJson(metadata) }}</pre>
    </div>
  </div>
</template>
