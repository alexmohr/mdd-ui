<script setup lang="ts">
import { ref } from "vue";
import { useAppStore } from "../stores/app";
import type { VisibleNode } from "../api/commands";

const store = useAppStore();
const showLegend = ref(false);

const legend = [
  { label: "Section", cls: "bg-gray-100" },
  { label: "Container / Variant", cls: "bg-sky-300" },
  { label: "Service / Job", cls: "bg-violet-400" },
  { label: "Request", cls: "bg-teal-400" },
  { label: "Pos-Response", cls: "bg-emerald-400" },
  { label: "Neg-Response", cls: "bg-rose-400" },
  { label: "Functional Class", cls: "bg-orange-400" },
  { label: "DOP", cls: "bg-pink-400" },
  { label: "SDG", cls: "bg-lime-400" },
  { label: "Inherited (parent ref)", cls: "bg-gray-500" },
];

const diffLegend = [
  { label: "Added", cls: "bg-emerald-400" },
  { label: "Removed", cls: "bg-red-400" },
  { label: "Modified", cls: "bg-amber-400" },
  { label: "Unchanged", cls: "bg-gray-500" },
];

function nodeTextClass(node: VisibleNode): string {
  if (node.diff_status === "Added") return "text-emerald-400";
  if (node.diff_status === "Removed") return "text-red-400/80 line-through";
  if (node.diff_status === "Modified") return "text-amber-300";
  if (node.diff_status === "Unchanged") return "text-gray-500/70";

  switch (node.node_type) {
    case "SectionHeader": return "text-gray-100 font-semibold";
    case "Container": return "text-sky-300 font-medium";
    case "Service": return "text-violet-300";
    case "Job": return "text-violet-300/80";
    case "ParentRefService": return "text-gray-500 italic";
    case "Request": return "text-teal-300";
    case "PosResponse": return "text-emerald-300";
    case "NegResponse": return "text-rose-300";
    case "FunctionalClass": return "text-orange-300";
    case "Dop": return "text-pink-300";
    case "Sdg": return "text-lime-300";
    case "ParentRefs": return "text-gray-300 font-medium";
    default: return "text-gray-300";
  }
}

function nodeIcon(node: VisibleNode): { char: string; cls: string } {
  switch (node.node_type) {
    case "SectionHeader": return { char: "#", cls: "text-gray-500" };
    case "Container": return { char: "", cls: "" };
    case "Service": return { char: "S", cls: "text-violet-500" };
    case "Job": return { char: "J", cls: "text-violet-500/70" };
    case "ParentRefService": return { char: "S", cls: "text-gray-600" };
    case "Request": return { char: "Rq", cls: "text-teal-500" };
    case "PosResponse": return { char: "R+", cls: "text-emerald-500" };
    case "NegResponse": return { char: "R-", cls: "text-rose-500" };
    case "FunctionalClass": return { char: "FC", cls: "text-orange-500" };
    case "Dop": return { char: "D", cls: "text-pink-500" };
    case "Sdg": return { char: "G", cls: "text-lime-500" };
    default: return { char: "", cls: "" };
  }
}

async function onClick(node: VisibleNode) {
  await store.selectNode(node.index);
}

async function onDblClick(node: VisibleNode) {
  if (node.has_children) {
    await store.toggleExpand(node.index);
  }
}

async function onChevronClick(e: Event, node: VisibleNode) {
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
        class="p-1 rounded transition-colors"
        :class="showLegend ? 'text-blue-400 bg-blue-500/10' : 'text-gray-600 hover:text-gray-400 hover:bg-gray-800/60'"
        title="Toggle legend"
        @click="showLegend = !showLegend"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
      </button>
      <button
        class="p-1 rounded text-gray-600 hover:text-gray-400 hover:bg-gray-800/60 transition-colors"
        title="Sort (s)"
        @click="store.toggleSort()"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="m21 8-4-4-4 4"/><path d="M17 4v16"/></svg>
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

    <!-- Legend panel -->
    <div v-if="showLegend" class="border-b border-gray-800/60 px-3 py-2 bg-[#0a0c12] shrink-0">
      <div class="text-[10px] text-gray-500 uppercase tracking-wider font-medium mb-1.5">Node types</div>
      <div class="flex flex-wrap gap-x-3 gap-y-1">
        <div v-for="item in legend" :key="item.label" class="flex items-center gap-1.5">
          <span class="w-2 h-2 rounded-full shrink-0" :class="item.cls" />
          <span class="text-[11px] text-gray-500">{{ item.label }}</span>
        </div>
      </div>
      <template v-if="store.isDiff">
        <div class="text-[10px] text-gray-500 uppercase tracking-wider font-medium mt-2 mb-1.5">Diff status</div>
        <div class="flex flex-wrap gap-x-3 gap-y-1">
          <div v-for="item in diffLegend" :key="item.label" class="flex items-center gap-1.5">
            <span class="w-2 h-2 rounded-full shrink-0" :class="item.cls" />
            <span class="text-[11px] text-gray-500">{{ item.label }}</span>
          </div>
        </div>
      </template>
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
          ? 'bg-blue-600/15'
          : 'hover:bg-gray-800/40'"
        :style="{ paddingLeft: `${node.depth * 14 + 6}px` }"
        @click="onClick(node)"
        @dblclick="onDblClick(node)"
      >
        <!-- Expand toggle -->
        <span
          v-if="node.has_children"
          class="w-4 h-4 flex items-center justify-center text-gray-600 group-hover:text-gray-400 shrink-0 transition-transform"
          :class="node.expanded ? 'rotate-0' : '-rotate-90'"
          @click="onChevronClick($event, node)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><path d="m7 10 5 5 5-5z"/></svg>
        </span>
        <span
          v-else-if="nodeIcon(node).char"
          class="w-4 shrink-0 text-center text-[9px] font-bold leading-none"
          :class="nodeIcon(node).cls"
        >{{ nodeIcon(node).char }}</span>
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
