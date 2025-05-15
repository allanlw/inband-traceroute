// This file has been renamed to Server.vue
<script setup lang="ts">
import type { Node } from '@/services/traceApi'
import Trace from './Trace.vue'

defineProps<{
  node: Node
  nodeId: string
}>()
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-lg font-medium text-gray-900">{{ nodeId.toUpperCase() }}</h2>
        <p class="text-sm text-gray-500 font-mono">{{ node.dns_name }}</p>
      </div>
    </div>
    <div class="flex flex-row gap-4">
      <Trace :node="node" :nodeId="nodeId" protocol="IPv4" class="flex-1" />
      <Trace :node="node" :nodeId="nodeId" protocol="IPv6" class="flex-1" />
    </div>
  </div>
</template>
