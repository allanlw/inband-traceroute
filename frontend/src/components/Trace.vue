<script setup lang="ts">
import type { Node } from '@/services/traceApi'
import HopDisplay from './HopDisplay.vue'
import { ref, onMounted, onUnmounted, toRefs } from 'vue'
import { traceApi } from '@/services/traceApi'
import { TraceConnection } from '@/services/traceConnection'

const props = defineProps<{
  node: Node
  nodeId: string
  protocol: 'IPv4' | 'IPv6'
}>()
const { node, protocol } = toRefs(props)

const hopEntries = ref<Array<[number, any]>>([])
const reverseDnsMap = ref<{ [ttl: number]: any }>({})
let connection: TraceConnection | null = null

function setupConnection() {
  connection = new TraceConnection(node.value, protocol.value, (n, p) => {
    // Use the same logic as before for node URL
    const baseDomain = n.dns_name
    const prefix = p === 'ipv4' ? 'ipv4.' : 'ipv6.'
    return `https://${prefix}${baseDomain}/sse`
  })
  connection.connect()
  connection.onUpdate(({ hops, reverseDnsMap: rmap }) => {
    hopEntries.value = hops
    reverseDnsMap.value = rmap
  })
}

onMounted(setupConnection)
onUnmounted(() => { connection?.disconnect() })
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
          <HopDisplay :message="hop" :reverseDns="reverseDnsMap[ttl]" />
        </div>
      </div>
    </div>
  </div>
</template>
