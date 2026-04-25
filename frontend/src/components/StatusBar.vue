<script setup lang="ts">
import { useAppStore } from "../stores/app";

const store = useAppStore();
</script>

<template>
  <div class="flex items-center h-6 px-3 bg-neutral-900 border-t border-neutral-800/40 text-[0.85em] text-neutral-500 shrink-0">
    <span v-if="store.loading" class="text-blue-400 animate-pulse mr-2">Loading...</span>
    <span v-if="store.status" class="mr-3">{{ store.status }}</span>
    <span class="flex-1" />
    <span v-if="store.nodes.length" class="mr-3">{{ store.nodes.length }} visible</span>
    <span v-if="store.isDiff" class="px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-500/80 text-[0.77em] font-medium">DIFF</span>
  </div>
</template>
