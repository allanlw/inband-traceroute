<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

interface HopData {
  number: number
  host: string
  loss: number
  sent: number
  last: number
  avg: number
  best: number
  worst: number
  stdev: number
}

interface Node {
  dns_name: string
  ipv4: string
  ipv6: string
}

interface NodeMap {
  [key: string]: Node
}

interface NodeConnection {
  id: string
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  lastMessage?: string
  lastUpdate: Date
  error?: string
}

const targetHost = ref('')
const isTracing = ref(false)
const nodes = ref<NodeMap>({})
const nodeConnections = ref<Record<string, NodeConnection>>({})
const isLoadingNodes = ref(false)
const nodeError = ref<string | null>(null)
const eventSources = new Map<string, EventSource>()

const hops = ref<HopData[]>([
  {
    number: 1,
    host: '192.168.1.1',
    loss: 0,
    sent: 10,
    last: 1.2,
    avg: 1.5,
    best: 1.0,
    worst: 2.1,
    stdev: 0.3
  },
  {
    number: 2,
    host: '10.0.0.1',
    loss: 10,
    sent: 10,
    last: 5.6,
    avg: 6.2,
    best: 4.8,
    worst: 8.9,
    stdev: 1.2
  }
])

const connectToNode = (nodeId: string, node: Node) => {
  if (eventSources.has(nodeId)) {
    return
  }

  nodeConnections.value[nodeId] = {
    id: nodeId,
    status: 'connecting',
    lastUpdate: new Date()
  }

  const url = `https://${node.dns_name}/sse`

  try {
    const eventSource = new EventSource(url)

    eventSource.onopen = () => {
      nodeConnections.value[nodeId] = {
        ...nodeConnections.value[nodeId],
        status: 'connected',
        lastUpdate: new Date()
      }
    }

    eventSource.onmessage = (event) => {
      nodeConnections.value[nodeId] = {
        ...nodeConnections.value[nodeId],
        lastMessage: event.data,
        lastUpdate: new Date()
      }
    }

    eventSource.onerror = (error) => {
      nodeConnections.value[nodeId] = {
        ...nodeConnections.value[nodeId],
        status: 'error',
        error: 'Connection failed',
        lastUpdate: new Date()
      }
      eventSource.close()
      eventSources.delete(nodeId)
    }

    eventSources.set(nodeId, eventSource)
  } catch (error) {
    nodeConnections.value[nodeId] = {
      ...nodeConnections.value[nodeId],
      status: 'error',
      error: error instanceof Error ? error.message : 'Failed to connect',
      lastUpdate: new Date()
    }
  }
}

const disconnectFromNode = (nodeId: string) => {
  const eventSource = eventSources.get(nodeId)
  if (eventSource) {
    eventSource.close()
    eventSources.delete(nodeId)
    nodeConnections.value[nodeId] = {
      ...nodeConnections.value[nodeId],
      status: 'disconnected',
      lastUpdate: new Date()
    }
  }
}

const startTrace = () => {
  if (!targetHost.value) return
  isTracing.value = true
  // TODO: Implement actual tracing logic
  console.log(`Starting trace to ${targetHost.value}`)
}

const fetchNodes = async () => {
  isLoadingNodes.value = true
  nodeError.value = null
  try {
    const response = await fetch('https://inband-traceroute.net/nodes.json')
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    nodes.value = await response.json()

    // Connect to all nodes
    Object.entries(nodes.value).forEach(([nodeId, node]) => {
      connectToNode(nodeId, node)
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
  // Clean up all connections
  eventSources.forEach((es, nodeId) => {
    disconnectFromNode(nodeId)
  })
})
</script>

<template>
  <div class="p-4">
    <div class="max-w-6xl mx-auto">
      <div class="mb-6 space-y-4">
        <!-- Node status section -->
        <div class="bg-white rounded-lg shadow-sm p-4">
          <div class="flex items-center justify-between mb-3">
            <h2 class="text-lg font-semibold">Available Nodes</h2>
            <button
              @click="fetchNodes"
              class="text-sm text-blue-500 hover:text-blue-600"
              :disabled="isLoadingNodes"
            >
              Refresh
            </button>
          </div>
          <div v-if="isLoadingNodes" class="text-gray-500">
            Loading nodes...
          </div>
          <div v-else-if="nodeError" class="text-red-500">
            {{ nodeError }}
          </div>
          <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div
              v-for="(node, location) in nodes"
              :key="location"
              class="p-4 border rounded-md bg-gray-50"
            >
              <div class="flex items-center justify-between mb-2">
                <span class="font-medium uppercase">{{ location }}</span>
                <span
                  :class="{
                    'text-green-500': nodeConnections[location]?.status === 'connected',
                    'text-yellow-500': nodeConnections[location]?.status === 'connecting',
                    'text-red-500': nodeConnections[location]?.status === 'error',
                    'text-gray-400': nodeConnections[location]?.status === 'disconnected'
                  }"
                  class="text-sm flex items-center gap-2"
                >
                  ● {{ nodeConnections[location]?.status || 'unknown' }}
                </span>
              </div>
              <div class="space-y-1">
                <div class="text-sm">
                  <span class="text-gray-500">DNS:</span>
                  <span class="ml-2 font-mono">{{ node.dns_name }}</span>
                </div>
                <div class="text-sm">
                  <span class="text-gray-500">IPv4:</span>
                  <span class="ml-2 font-mono">{{ node.ipv4 }}</span>
                </div>
                <div class="text-sm">
                  <span class="text-gray-500">IPv6:</span>
                  <span class="ml-2 font-mono text-xs">{{ node.ipv6 }}</span>
                </div>
                <div v-if="nodeConnections[location]?.lastMessage" class="mt-2 p-2 bg-gray-100 rounded text-sm">
                  <div class="text-gray-500">Last message:</div>
                  <div class="font-mono text-xs break-all">{{ nodeConnections[location].lastMessage }}</div>
                  <div class="text-gray-400 text-xs mt-1">
                    {{ new Date(nodeConnections[location].lastUpdate).toLocaleString() }}
                  </div>
                </div>
                <div v-if="nodeConnections[location]?.error" class="mt-2 text-sm text-red-500">
                  {{ nodeConnections[location].error }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Existing input section -->
        <div class="bg-white rounded-lg shadow-sm p-4">
          <div class="flex gap-4">
            <input
              v-model="targetHost"
              type="text"
              placeholder="Enter hostname or IP address"
              class="flex-1 px-4 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              :disabled="isTracing"
              @keyup.enter="startTrace"
            />
            <button
              @click="startTrace"
              :disabled="isTracing || !targetHost"
              class="px-6 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {{ isTracing ? 'Tracing...' : 'Start Trace' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Existing table section -->
      <div class="bg-white rounded-lg shadow-sm overflow-hidden">
        <div class="overflow-x-auto">
          <table class="min-w-full divide-y divide-gray-200">
            <thead class="bg-gray-50">
              <tr>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Hop</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Host</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Loss %</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Sent</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Last</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Avg</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Best</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Worst</th>
                <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">StDev</th>
              </tr>
            </thead>
            <tbody class="bg-white divide-y divide-gray-200">
              <tr v-for="hop in hops" :key="hop.number" class="hover:bg-gray-50">
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.number }}</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 font-mono">{{ hop.host }}</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.loss.toFixed(1) }}%</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.sent }}</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.last.toFixed(1) }}ms</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.avg.toFixed(1) }}ms</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.best.toFixed(1) }}ms</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.worst.toFixed(1) }}ms</td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{{ hop.stdev.toFixed(1) }}ms</td>
              </tr>
              <tr v-if="hops.length === 0" class="hover:bg-gray-50">
                <td colspan="9" class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 text-center">
                  No trace results yet. Enter a hostname or IP address to start tracing.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
