// This file has been renamed to Server.vue
<script setup lang="ts">
import type { Node, NodeConnection, TraceData } from '@/services/traceApi'
import Trace from './Trace.vue'

defineProps<{
  node: Node
  nodeId: string
  connection: {
    ipv4: NodeConnection
    ipv6: NodeConnection
  }
  traceData: TraceData
}>()
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-lg font-medium text-gray-900">{{ nodeId.toUpperCase() }}</h2>
        <p class="text-sm text-gray-500 font-mono">{{ node.dns_name }}</p>
      </div>
      <div class="flex gap-3">
        <span
          :class="{
            'text-green-500': connection.ipv4?.status === 'connected',
            'text-yellow-500': connection.ipv4?.status === 'connecting',
            'text-red-500': connection.ipv4?.status === 'error',
            'text-gray-400': connection.ipv4?.status === 'disconnected'
          }"
          class="text-sm flex items-center gap-1"
        >
          ● v4
        </span>
        <span
          :class="{
            'text-green-500': connection.ipv6?.status === 'connected',
            'text-yellow-500': connection.ipv6?.status === 'connecting',
            'text-red-500': connection.ipv6?.status === 'error',
            'text-gray-400': connection.ipv6?.status === 'disconnected'
          }"
          class="text-sm flex items-center gap-1"
        >
          ● v6
        </span>
      </div>
    </div>
    <div class="flex flex-row gap-4">
      <Trace :traceData="traceData" :nodeId="nodeId" protocol="IPv4" class="flex-1" />
      <Trace :traceData="traceData" :nodeId="nodeId" protocol="IPv6" class="flex-1" />
    </div>
  </div>
</template>
