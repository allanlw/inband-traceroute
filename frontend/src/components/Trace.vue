<script setup lang="ts">
import type { Node } from '@/services/traceApi';
import HopDisplay from './HopDisplay.vue';
import { ref, onMounted, onUnmounted, toRefs } from 'vue';
import { traceApi } from '@/services/traceApi';
import { TraceConnection } from '@/services/traceConnection';

const props = defineProps<{
  node: Node;
  nodeId: string;
  protocol: 'IPv4' | 'IPv6';
}>();
const { node, protocol } = toRefs(props);

const hopEntries = ref<Array<[number, any]>>([]);
const reverseDnsMapRef = ref<{ [ttl: number]: any }>({});
const traceStatus = ref<'not-started' | 'in-progress' | 'done'>('not-started');
let connection: TraceConnection | null = null;

function setupConnection() {
  connection = new TraceConnection(node.value, protocol.value, (n, p) => {
    // Use the same logic as before for node URL
    const baseDomain = n.dns_name;
    const prefix = p === 'ipv4' ? 'ipv4.' : 'ipv6.';
    return `https://${prefix}${baseDomain}/sse`;
  });
  connection.connect();
  connection.addUpdateListener(({ hops, reverseDnsMap, status }) => {
    hopEntries.value = hops;
    reverseDnsMapRef.value = { ...reverseDnsMap };
    traceStatus.value = status;
  });
}

onMounted(() => {
  traceStatus.value = 'not-started';
  setupConnection();
});
onUnmounted(() => {
  connection?.disconnect();
});
</script>

<template>
  <div class="rounded-lg overflow-hidden bg-white w-full">
    <div class="px-4 py-2 bg-gray-50 border-b flex items-center gap-4">
      <h3 class="text-sm font-medium text-gray-700">{{ protocol }}</h3>
      <span v-if="traceStatus === 'not-started'" class="text-xs text-gray-400">Not started</span>
      <span v-else-if="traceStatus === 'in-progress'" class="text-xs text-blue-500 animate-pulse"
        >In progress…</span
      >
      <span v-else-if="traceStatus === 'done'" class="text-xs text-green-600">Done</span>
    </div>
    <div class="divide-y divide-gray-100">
      <div
        v-for="([ttl, hop], idx) in hopEntries"
        :key="ttl"
        class="px-2 py-1 flex items-center gap-2 min-h-[36px]"
      >
        <div class="flex-1">
          <HopDisplay :message="hop" :reverseDns="reverseDnsMapRef[ttl]" />
        </div>
      </div>
    </div>
  </div>
</template>
