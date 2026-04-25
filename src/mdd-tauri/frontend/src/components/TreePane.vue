<script setup lang="ts">
import { useAppStore } from "../stores/app";
import type { VisibleNode } from "../api/commands";

const store = useAppStore();

function nodeTextClass(node: VisibleNode): string {
  if (node.diff_status === "Added") return "text-emerald-400";
  if (node.diff_status === "Removed") return "text-red-400/70 line-through";
  if (node.diff_status === "Modified") return "text-amber-400";
  if (node.diff_status === "Unchanged") return "text-gray-600";
  if (node.node_type === "Container") return "text-sky-300 font-semibold";
  if (node.node_type === "SectionHeader") return "text-gray-200 font-semibold";
  if (node.node_type === "ParentRefService") return "text-gray-600 italic";
  if (node.node_type === "Service" || node.node_type === "Job") return "text-gray-300";
  return "text-gray-400";
}

async function onClick(node: VisibleNode) {
  await store.selectNode(node.index);
}

async function onToggle(e: Event, node: VisibleNode) {
  e.stopPropagation();
  if (node.has_children) {
    await store.toggleExpand(node.index);
  }
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#0c0e14]">
    <!-- Header -->
    <div class="flex items-center h-8 px-2 border-b border-gray-800/60 shrink-0 gap-1">
      <span class="text-[11px] text-gray-500 font-medium uppercase tracking-wider flex-1">Explorer</span>
      <button
        class="p-1 rounded text-gray-600 hover:text-gray-400 hover:bg-gray-800/60 transition-colors"
        title="Search (/)"
        @click="store.searchActive = true"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
      </button>
      <button
        class="p-1 rounded text-gray-600 hover:text-gray-400 hover:bg-gray-800/60 transition-colors"
        title="Expand all (e)"
        @click="store.expandAll()"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>
      </button>
      <button
        class="p-1 rounded text-gray-600 hover:text-gray-400 hover:bg-gray-800/60 transition-colors"
        title="Collapse all (c)"
        @click="store.collapseAll()"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 20 5-5 5 5"/><path d="m7 4 5 5 5-5"/></svg>
      </button>
    </div>

    <!-- Node list -->
    <div class="flex-1 overflow-y-auto overflow-x-hidden py-0.5">
      <div v-if="store.nodes.length === 0" class="text-gray-700 text-center text-xs mt-12">
        No nodes loaded
      </div>
      <div
        v-for="node in store.nodes"
        :key="node.index"
        class="flex items-center h-[22px] cursor-pointer transition-colors group"
        :class="node.index === store.selectedIndex
          ? 'bg-blue-600/15 text-gray-200'
          : 'hover:bg-gray-800/40'"
        :style="{ paddingLeft: `${node.depth * 14 + 6}px` }"
        @click="onClick(node)"
      >
        <!-- Expand toggle -->
        <span
          v-if="node.has_children"
          class="w-4 h-4 flex items-center justify-center text-gray-600 group-hover:text-gray-400 shrink-0 transition-transform"
          :class="node.expanded ? 'rotate-0' : '-rotate-90'"
          @click="onToggle($event, node)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><path d="m7 10 5 5 5-5z"/></svg>
        </span>
        <span v-else class="w-4 shrink-0" />

        <!-- Label -->
        <span
          class="truncate text-[12px] leading-tight"
          :class="nodeTextClass(node)"
        >{{ node.text }}</span>
      </div>
    </div>
  </div>
</template>
