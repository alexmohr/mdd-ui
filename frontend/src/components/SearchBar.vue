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
  <div class="flex items-center gap-2 h-9 px-3 bg-neutral-900 border-b border-neutral-800/60 shrink-0">
    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-neutral-500 shrink-0"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
    <input
      ref="input"
      v-model="store.searchQuery"
      class="flex-1 bg-transparent text-neutral-200 text-[1em] outline-none placeholder-neutral-600"
      placeholder="Search nodes..."
      @keydown.enter="onSubmit"
      @keydown.escape="onCancel"
    />
    <select
      :value="store.searchScope"
      class="bg-neutral-800 text-neutral-300 text-[0.85em] rounded px-1.5 py-0.5 border border-neutral-700 outline-none cursor-pointer hover:border-neutral-500 transition-colors"
      @change="store.setScope(($event.target as HTMLSelectElement).value)"
    >
      <option value="All">All</option>
      <option value="Variants">Variants</option>
      <option value="Functional Groups">Functional Groups</option>
      <option value="ECU Shared Data">ECU Shared Data</option>
      <option value="Services">Services</option>
      <option value="Diag-Comms">Diag-Comms</option>
      <option value="Requests">Requests</option>
      <option value="Responses">Responses</option>
    </select>
    <button
      class="p-1 rounded text-neutral-500 hover:text-red-400 hover:bg-neutral-800 transition-colors"
      title="Clear search (x)"
      @click="onClear"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
    </button>
  </div>
</template>
