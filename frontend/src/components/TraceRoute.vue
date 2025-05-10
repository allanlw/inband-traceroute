<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { traceApi } from '@/services/traceApi'
import type { Node, NodeMap, NodeConnectionMap, TraceMessage, IpVersion } from '@/services/traceApi'
import HopDisplay from './HopDisplay.vue'
import Server from './Server.vue'

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
  return Array.from({ length: maxTtl.value + 1 }, (_, i) => i + 1).filter(i => i > 0).reverse()
})


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
    <div class="max-w-7xl mx-auto">
      <div class="space-y-8">
        <div v-for="(node, nodeId) in nodes" :key="nodeId">
          <Server
            :node="node"
            :nodeId="nodeId"
            :connection="nodeConnections[nodeId]"
            :traceData="traceData"
          />
        </div>
      </div>
    </div>
  </div>
</template>
