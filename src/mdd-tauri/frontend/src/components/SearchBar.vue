<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAppStore } from "../stores/app";

const store = useAppStore();
const input = ref<HTMLInputElement | null>(null);

onMounted(() => input.value?.focus());

async function onSubmit() {
  if (store.searchQuery.trim()) {
    await store.search(store.searchQuery.trim());
    store.searchQuery = "";
  }
  store.searchActive = false;
}

function onCancel() {
  store.searchQuery = "";
  store.searchActive = false;
}

async function onClear() {
  await store.clearSearch();
  store.searchQuery = "";
  store.searchActive = false;
}
</script>

<template>
  <div class="flex items-center gap-2 h-9 px-3 bg-[#12151e] border-b border-gray-800/60 shrink-0">
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-gray-600 shrink-0"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
    <input
      ref="input"
      v-model="store.searchQuery"
      class="flex-1 bg-transparent text-gray-300 text-[12px] outline-none placeholder-gray-700"
      placeholder="Search nodes..."
      @keydown.enter="onSubmit"
      @keydown.escape="onCancel"
    />
    <button
      class="px-2 py-0.5 rounded text-[11px] text-gray-600 hover:text-gray-400 hover:bg-gray-800/60 transition-colors"
      @click="store.cycleScope()"
    >{{ store.searchScope }}</button>
    <button
      class="p-1 rounded text-gray-600 hover:text-red-400 hover:bg-gray-800/60 transition-colors"
      title="Clear search (x)"
      @click="onClear"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
    </button>
  </div>
</template>
