<script setup lang="ts">
import type { TraceMessage } from '@/services/traceApi'
import { traceApi } from '@/services/traceApi'

defineProps<{
  message?: TraceMessage
}>()

const formatCountry = (message: TraceMessage | undefined) => {
  if (!message?.enriched_info) return ''
  const { country, as_domain } = message.enriched_info
  const parts = []
  if (country) parts.push(country)
  if (as_domain) parts.push(as_domain)
  return parts.join(' - ')
}

const isTimeout = (message: TraceMessage | undefined) => {
  return message?.hop_type === 'Timeout'
}
</script>

<template>
  <div v-if="message" class="flex flex-col justify-center min-h-[36px] py-1 px-2">
    <template v-if="isTimeout(message)">
      <div class="flex items-center justify-between w-full">
        <div class="text-xs text-gray-400 mr-2">timeout</div>
        <div class="text-gray-400 italic">* * *</div>
      </div>
    </template>
    <template v-else>
      <div class="flex items-center justify-between w-full">
        <div class="font-mono truncate max-w-[120px]">
          <template v-if="message.addr">
            <span v-if="message.addr.includes(':')">
              {{ message.addr.length > 18 ? message.addr.slice(0, 8) + '…' + message.addr.slice(-6) : message.addr }}
            </span>
            <span v-else>{{ message.addr }}</span>
          </template>
          <span v-else class="text-gray-400">-</span>
        </div>
        <div class="text-xs text-gray-500 ml-2 whitespace-nowrap text-right">
          {{ traceApi.formatRtt(message.rtt || 0) }}ms
        </div>
      </div>
      <div class="text-xs text-gray-600 truncate max-w-full" style="max-width: 180px;">
        {{ formatCountry(message).length > 32 ? formatCountry(message).slice(0, 29) + '…' : formatCountry(message) }}
      </div>
    </template>
  </div>
  <span v-else class="text-gray-400">-</span>
</template>
