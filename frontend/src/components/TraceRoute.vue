<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { traceApi } from '@/services/traceApi'
import type { NodeMap } from '@/services/traceApi'
import Server from './Server.vue'

const nodes = ref<NodeMap>({})
const isLoadingNodes = ref(false)
const nodeError = ref<string | null>(null)

const fetchNodes = async () => {
  isLoadingNodes.value = true
  nodeError.value = null
  try {
    nodes.value = await traceApi.fetchNodes()
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
</script>

<template>
  <div class="p-4">
    <div class="max-w-7xl mx-auto">
      <div class="flex flex-row flex-wrap gap-8 items-start">
        <div v-for="(node, nodeId) in nodes" :key="nodeId" class="min-w-[350px] flex-1">
          <Server :node="node" :nodeId="String(nodeId)" />
        </div>
      </div>
    </div>
  </div>
</template>
