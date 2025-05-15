<script setup lang="ts">
import type { TraceMessage, ReverseDnsMessage, EnrichedInfo } from '@/services/traceApi';

const props = defineProps<{
  message?: TraceMessage;
  reverseDns?: ReverseDnsMessage;
}>();

console.log('HopDisplay props:', { message: props.message, reverseDns: props.reverseDns });

const isTimeout = (message: TraceMessage | undefined) => {
  return message?.hop_type === 'Timeout';
};

const getReverseDns = (reverseDns: ReverseDnsMessage | undefined) => {
  if (!reverseDns) return '';
  if (reverseDns.name?.Ok) return reverseDns.name.Ok;
  if (reverseDns.name?.Err) return `[error: ${reverseDns.name.Err}]`;
  return '';
};

const formatRtt = (rtt: number) => {
  return (rtt / 1_000_000).toFixed(2); // Convert nanoseconds to milliseconds
};

const getCountryDisplay = (enriched: EnrichedInfo | null) => {
  if (!enriched?.country_code) return null;
  return {
    code: enriched.country_code,
    name: enriched.country || enriched.country_code,
  };
};

const getAsnDisplay = (enriched: EnrichedInfo | null) => {
  if (!enriched?.asn) return null;
  return {
    asn: enriched.asn,
    asName: enriched.as_name,
    asDomain: enriched.as_domain,
  };
};
</script>

<template>
  <div class="flex flex-row items-center gap-2 min-h-[36px] py-1 px-2 w-full max-w-full text-xs">
    <template v-if="!message">
      <span class="text-gray-400 w-[32px] text-center">-</span>
      <span class="text-gray-400 w-[80px] text-center">-</span>
      <span class="text-gray-400 w-[40px] text-center">-</span>
      <span class="text-gray-400 w-[40px] text-center">-</span>
      <span class="text-gray-400 w-[80px] text-center">-</span>
    </template>
    <template v-else-if="isTimeout(message)">
      <span class="text-gray-400 w-[32px] text-center">{{ message.ttl ?? '-' }}</span>
      <span class="text-gray-400 w-[80px] text-center">timeout</span>
      <span class="text-gray-400 italic w-[40px] text-center">* * *</span>
      <span class="text-gray-400 w-[40px] text-center">-</span>
      <span class="text-gray-400 w-[80px] text-center">-</span>
    </template>
    <template v-else>
      <!-- TTL -->
      <span class="w-[32px] text-center">{{ message.ttl ?? '-' }}</span>
      <!-- Address -->
      <span class="font-mono truncate w-[80px]" :title="message.addr || '-'">
        <template v-if="message.addr">
          <span v-if="message.addr.includes(':')">
            {{
              message.addr.length > 18
                ? message.addr.slice(0, 8) + '…' + message.addr.slice(-6)
                : message.addr
            }}
          </span>
          <span v-else>{{ message.addr }}</span>
        </template>
        <span v-else class="text-gray-400">-</span>
      </span>
      <!-- RTT -->
      <span
        class="text-gray-500 w-[48px] text-right"
        :title="message.rtt ? formatRtt(message.rtt) + ' ms' : '-'"
      >
        <template v-if="message.rtt !== undefined && message.rtt !== null">
          {{ formatRtt(message.rtt) }}
        </template>
        <span v-else class="text-gray-400">-</span>
      </span>
      <!-- Country -->
      <span class="w-[40px] text-center">
        <template v-if="getCountryDisplay(message.enriched_info)">
          <abbr :title="getCountryDisplay(message.enriched_info)?.name">
            {{ getCountryDisplay(message.enriched_info)?.code }}
          </abbr>
        </template>
        <span v-else class="text-gray-400">-</span>
      </span>
      <!-- ASN -->
      <span class="w-[80px] text-center">
        <template v-if="getAsnDisplay(message.enriched_info)">
          <abbr
            :title="
              [
                getAsnDisplay(message.enriched_info)?.asName,
                getAsnDisplay(message.enriched_info)?.asDomain,
              ]
                .filter(Boolean)
                .join(' | ')
            "
          >
            {{ getAsnDisplay(message.enriched_info)?.asn }}
          </abbr>
        </template>
        <span v-else class="text-gray-400">-</span>
      </span>
    </template>
  </div>
  <div
    v-if="reverseDns && reverseDns.name && getReverseDns(reverseDns)"
    class="text-xs text-blue-700 truncate max-w-full pl-2"
    style="max-width: 180px"
  >
    {{ getReverseDns(reverseDns) }}
  </div>
</template>
