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
  <div class="flex items-center gap-2 px-3 py-1 bg-gray-800 border-b border-gray-700">
    <span class="text-yellow-500 text-xs">/</span>
    <input
      ref="input"
      v-model="store.searchQuery"
      class="flex-1 bg-transparent text-gray-200 text-xs outline-none placeholder-gray-600"
      placeholder="Search..."
      @keydown.enter="onSubmit"
      @keydown.escape="onCancel"
    />
    <span class="text-gray-600 text-xs">scope: {{ store.searchScope }}</span>
    <button
      class="text-xs text-gray-500 hover:text-gray-300 px-1"
      @click="store.cycleScope()"
    >
      ⇄
    </button>
    <button
      class="text-xs text-red-500 hover:text-red-300 px-1"
      @click="onClear"
    >
      ✕
    </button>
  </div>
</template>
