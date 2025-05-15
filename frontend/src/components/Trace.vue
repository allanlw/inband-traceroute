<script setup lang="ts">
import type { TraceData, TraceMessage } from '@/services/traceApi'
import HopDisplay from './HopDisplay.vue'
import { computed } from 'vue'

const props = defineProps<{ traceData: TraceData; nodeId: string; protocol: 'IPv4' | 'IPv6' }>()

const hopEntries = computed(() => {
  const protoKey = props.protocol === 'IPv4' ? 'ipv4' : 'ipv6'
  const entries: Array<[number, TraceMessage]> = []
  Object.entries(props.traceData).forEach(([ttl, data]) => {
    if (data[props.nodeId]?.[protoKey]) {
      entries.push([Number(ttl), data[props.nodeId][protoKey]])
    }
  })
  // Sort by TTL descending (last event on top)
  return entries.sort((a, b) => b[0] - a[0])
})
</script>

<template>
  <div class="rounded-lg overflow-hidden bg-white w-full">
    <div class="px-4 py-2 bg-gray-50 border-b">
      <h3 class="text-sm font-medium text-gray-700">{{ protocol }}</h3>
    </div>
    <div class="divide-y divide-gray-100">
      <div v-for="([ttl, hop], idx) in hopEntries" :key="ttl" class="px-2 py-1 flex items-center gap-2 min-h-[36px]">
        <div class="flex items-center mr-2">
          <div class="w-7 text-xs text-gray-500 flex-shrink-0 text-right">{{ ttl }}</div>
          <div class="h-6 w-px bg-blue-400 mx-2"></div>
        </div>
        <div class="flex-1">
          <HopDisplay :message="hop" />
        </div>
      </div>
    </div>
  </div>
</template>
