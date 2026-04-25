<script setup lang="ts">
import { useAppStore } from "../stores/app";

const store = useAppStore();
</script>

<template>
  <div class="px-3 py-0.5 bg-gray-900 border-t border-gray-800 text-xs text-gray-500 flex items-center gap-3">
    <span>{{ store.status }}</span>
    <span class="flex-1" />
    <span v-if="store.nodes.length">{{ store.nodes.length }} visible</span>
    <span v-if="store.isDiff" class="text-yellow-600">DIFF</span>
    <span v-if="store.loading" class="text-blue-400 animate-pulse">Loading...</span>
  </div>
</template>
