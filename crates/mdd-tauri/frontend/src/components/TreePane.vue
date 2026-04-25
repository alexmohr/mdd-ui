<script setup lang="ts">
import { useAppStore } from "../stores/app";

const store = useAppStore();

function nodeClass(node: { node_type: string; diff_status: string | null }) {
  const base = "cursor-pointer select-none py-px px-1 rounded-sm hover:bg-gray-800";
  if (node.diff_status === "Added") return `${base} text-green-400`;
  if (node.diff_status === "Removed") return `${base} text-red-400 line-through`;
  if (node.diff_status === "Modified") return `${base} text-yellow-400`;
  if (node.diff_status === "Unchanged") return `${base} text-gray-600`;

  if (node.node_type === "Container") return `${base} text-cyan-300 font-bold`;
  if (node.node_type === "SectionHeader") return `${base} text-blue-300 font-bold`;
  if (node.node_type === "ParentRefService") return `${base} text-gray-500`;
  return `${base} text-gray-300`;
}

function expandIcon(node: { has_children: boolean; expanded: boolean }) {
  if (!node.has_children) return "\u00A0\u00A0";
  return node.expanded ? "▼ " : "▶ ";
}

async function onClick(node: { index: number; has_children: boolean }) {
  await store.selectNode(node.index);
}

async function onToggle(e: Event, node: { index: number; has_children: boolean }) {
  e.stopPropagation();
  if (node.has_children) {
    await store.toggleExpand(node.index);
  }
}
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="px-2 py-1 bg-gray-900 border-b border-gray-800 text-xs text-gray-500 font-semibold uppercase tracking-wide">
      Tree
    </div>
    <div class="flex-1 overflow-y-auto overflow-x-hidden p-1">
      <div v-if="store.nodes.length === 0" class="text-gray-600 text-center mt-8">
        Open an MDD file to browse
      </div>
      <div
        v-for="node in store.nodes"
        :key="node.index"
        :class="[
          nodeClass(node),
          node.index === store.selectedIndex ? 'bg-blue-900/50 ring-1 ring-blue-700' : '',
        ]"
        :style="{ paddingLeft: `${node.depth * 16 + 4}px` }"
        @click="onClick(node)"
      >
        <span
          class="inline-block w-4 text-center text-gray-600 cursor-pointer"
          @click="onToggle($event, node)"
        >{{ expandIcon(node) }}</span>
        <span>{{ node.text }}</span>
      </div>
    </div>
  </div>
</template>
