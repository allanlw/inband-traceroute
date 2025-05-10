<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { traceApi } from '@/services/traceApi'
import type { Node, NodeMap, NodeConnectionMap, TraceMessage, IpVersion } from '@/services/traceApi'

interface TraceData {
  [ttl: number]: {
    [nodeId: string]: {
      ipv4?: TraceMessage
      ipv6?: TraceMessage
    }
  }
}

const targetHost = ref('')
const isTracing = ref(false)
const nodes = ref<NodeMap>({})
const nodeConnections = ref<NodeConnectionMap>({})
const isLoadingNodes = ref(false)
const nodeError = ref<string | null>(null)
const traceData = ref<TraceData>({})

const maxTtl = computed(() => {
  const ttls = Object.keys(traceData.value).map(Number)
  return ttls.length ? Math.max(...ttls) : 0
})

const ttlRange = computed(() => {
  return Array.from({ length: maxTtl.value + 1 }, (_, i) => i).filter(i => i > 0)
})

const formatCountry = (message: TraceMessage | undefined) => {
  if (!message?.enriched_info) return ''
  const { country, asn, as_name } = message.enriched_info
  const parts = []
  if (country) parts.push(country)
  if (asn) parts.push(`AS${asn}${as_name ? ` (${as_name})` : ''}`)
  return parts.join(' - ')
}

const onNodeMessage = (nodeId: string, ipVersion: IpVersion, message: TraceMessage) => {
  if (!nodeConnections.value[nodeId]) return

  // Update node connection state
  nodeConnections.value[nodeId][ipVersion] = {
    ...nodeConnections.value[nodeId][ipVersion],
    lastMessage: message,
    traceMessages: [...nodeConnections.value[nodeId][ipVersion].traceMessages, message].slice(-50),
    lastUpdate: new Date()
  }

  // Update trace data structure
  if (!traceData.value[message.ttl]) {
    traceData.value[message.ttl] = {}
  }
  if (!traceData.value[message.ttl][nodeId]) {
    traceData.value[message.ttl][nodeId] = {}
  }
  traceData.value[message.ttl][nodeId][ipVersion] = message
}

const onNodeStatusChange = (nodeId: string, ipVersion: IpVersion, status: NodeConnection['status'], error?: string) => {
  if (!nodeConnections.value[nodeId]) return

  nodeConnections.value[nodeId][ipVersion] = {
    ...nodeConnections.value[nodeId][ipVersion],
    status,
    error,
    lastUpdate: new Date()
  }
}

const fetchNodes = async () => {
  isLoadingNodes.value = true
  nodeError.value = null
  try {
    nodes.value = await traceApi.fetchNodes()

    // Connect to all nodes
    Object.entries(nodes.value).forEach(([nodeId, node]) => {
      // Initialize connection state if it doesn't exist
      if (!nodeConnections.value[nodeId]) {
        nodeConnections.value[nodeId] = {
          ipv4: {
            id: nodeId,
            ipVersion: 'ipv4',
            status: 'connecting',
            traceMessages: [],
            lastUpdate: new Date()
          },
          ipv6: {
            id: nodeId,
            ipVersion: 'ipv6',
            status: 'connecting',
            traceMessages: [],
            lastUpdate: new Date()
          }
        }
      }
      traceApi.connectToNode(nodeId, node, onNodeMessage, onNodeStatusChange)
    })
  } catch (error) {
    nodeError.value = error instanceof Error ? error.message : 'Failed to load nodes'
    console.error('Error fetching nodes:', error)
  } finally {
    isLoadingNodes.value = false
  }
}

onMounted(() => {
  fetchNodes()
})

onUnmounted(() => {
  traceApi.disconnectAll()
})
</script>

<template>
  <div class="p-4">
    <div class="max-w-[90rem] mx-auto">
      <div class="mb-6 space-y-4">
        <!-- Node status cards -->
        <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          <div
            v-for="(node, location) in nodes"
            :key="location"
            class="p-4 border rounded-md bg-gray-50"
          >
            <div class="flex items-center justify-between mb-2">
              <span class="font-medium uppercase">{{ location }}</span>
              <div class="flex gap-3">
                <span
                  :class="{
                    'text-green-500': nodeConnections[location]?.ipv4?.status === 'connected',
                    'text-yellow-500': nodeConnections[location]?.ipv4?.status === 'connecting',
                    'text-red-500': nodeConnections[location]?.ipv4?.status === 'error',
                    'text-gray-400': nodeConnections[location]?.ipv4?.status === 'disconnected'
                  }"
                  class="text-sm flex items-center gap-1"
                >
                  ● v4
                </span>
                <span
                  :class="{
                    'text-green-500': nodeConnections[location]?.ipv6?.status === 'connected',
                    'text-yellow-500': nodeConnections[location]?.ipv6?.status === 'connecting',
                    'text-red-500': nodeConnections[location]?.ipv6?.status === 'error',
                    'text-gray-400': nodeConnections[location]?.ipv6?.status === 'disconnected'
                  }"
                  class="text-sm flex items-center gap-1"
                >
                  ● v6
                </span>
              </div>
            </div>
            <div class="text-sm space-y-1">
              <div class="text-gray-500">DNS: <span class="ml-2 font-mono">{{ node.dns_name }}</span></div>
              <div class="text-gray-500">IPv4: <span class="ml-2 font-mono">{{ node.ipv4 }}</span></div>
              <div class="text-gray-500">IPv6: <span class="ml-2 font-mono text-xs">{{ node.ipv6 }}</span></div>
            </div>
          </div>
        </div>

        <!-- Trace data table -->
        <div class="bg-white rounded-lg shadow-sm overflow-hidden">
          <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-gray-200">
              <thead class="bg-gray-50">
                <tr>
                  <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">TTL</th>
                  <template v-for="(node, location) in nodes" :key="location">
                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      {{ location }} (v4)
                    </th>
                    <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                      {{ location }} (v6)
                    </th>
                  </template>
                </tr>
              </thead>
              <tbody class="bg-white divide-y divide-gray-200">
                <tr v-for="ttl in ttlRange" :key="ttl" class="hover:bg-gray-50">
                  <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ ttl }}</td>
                  <template v-for="(node, location) in nodes" :key="location">
                    <!-- IPv4 Cell -->
                    <td class="px-6 py-4 whitespace-nowrap text-sm">
                      <template v-if="traceData[ttl]?.[location]?.ipv4">
                        <div class="space-y-1">
                          <div class="font-mono">{{ traceData[ttl][location].ipv4?.addr }}</div>
                          <div class="text-xs text-gray-500">
                            {{ traceApi.formatRtt(traceData[ttl][location].ipv4?.rtt || 0) }}ms
                          </div>
                          <div class="text-xs text-gray-600">
                            {{ formatCountry(traceData[ttl][location].ipv4) }}
                          </div>
                        </div>
                      </template>
                      <template v-else>
                        <span class="text-gray-400">-</span>
                      </template>
                    </td>
                    <!-- IPv6 Cell -->
                    <td class="px-6 py-4 whitespace-nowrap text-sm">
                      <template v-if="traceData[ttl]?.[location]?.ipv6">
                        <div class="space-y-1">
                          <div class="font-mono">{{ traceData[ttl][location].ipv6?.addr }}</div>
                          <div class="text-xs text-gray-500">
                            {{ traceApi.formatRtt(traceData[ttl][location].ipv6?.rtt || 0) }}ms
                          </div>
                          <div class="text-xs text-gray-600">
                            {{ formatCountry(traceData[ttl][location].ipv6) }}
                          </div>
                        </div>
                      </template>
                      <template v-else>
                        <span class="text-gray-400">-</span>
                      </template>
                    </td>
                  </template>
                </tr>
                <tr v-if="!maxTtl" class="hover:bg-gray-50">
                  <td
                    :colspan="Object.keys(nodes).length * 2 + 1"
                    class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 text-center"
                  >
                    No trace data available yet.
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
