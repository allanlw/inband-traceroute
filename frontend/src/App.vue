<template>
  <div class="min-h-screen bg-gray-50">
    <header class="bg-white shadow-sm">
      <div class="max-w-6xl mx-auto px-4 py-4">
        <h1 class="text-2xl font-semibold text-gray-900">Inband Traceroute</h1>
      </div>
    </header>
    <main>
      <TraceRoute />
    </main>
  </div>
</template>

<script setup lang="ts">
import TraceRoute from './components/TraceRoute.vue';
</script>
