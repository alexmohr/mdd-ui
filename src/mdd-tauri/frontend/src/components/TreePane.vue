<script setup lang="ts">
import { ref, nextTick } from "vue";
import { useAppStore } from "../stores/app";
import type { VisibleNode } from "../api/commands";

const store = useAppStore();
const showLegend = ref(false);

// --- Context menu ---
const ctxMenu = ref<{ x: number; y: number; node: VisibleNode } | null>(null);

function onContextMenu(e: MouseEvent, node: VisibleNode) {
  e.preventDefault();
  ctxMenu.value = { x: e.clientX, y: e.clientY, node };
  nextTick(() => window.addEventListener("click", closeCtx, { once: true }));
}
function closeCtx() { ctxMenu.value = null; }

async function ctxAction(action: string) {
  const node = ctxMenu.value?.node;
  ctxMenu.value = null;
  if (!node) return;
  switch (action) {
    case "select": await store.selectNode(node.index); break;
    case "expand": if (node.has_children) await store.toggleExpand(node.index); break;
    case "expandAll": await store.expandAll(); break;
    case "collapseAll": await store.collapseAll(); break;
    case "goToParent":
      // Navigate to parent inherited-from (uses goBack as approximation for now)
      await store.selectNode(node.index);
      break;
    case "copyName":
      await navigator.clipboard.writeText(node.text);
      break;
    case "sort": await store.toggleSort(node.index); break;
  }
}

// --- Badges ---
type Badge = { label: string; bg: string; fg: string };

function nodeBadge(node: VisibleNode): Badge | null {
  // Node-type badges (both leaf and container)
  switch (node.node_type) {
    case "Service":          return { label: "SVC",  bg: "bg-violet-500/20",  fg: "text-violet-300" };
    case "Job":              return { label: "JOB",  bg: "bg-violet-500/15",  fg: "text-violet-300/70" };
    case "ParentRefService": return { label: "INH",  bg: "bg-amber-500/15",   fg: "text-amber-400" };
    case "Request":          return { label: "REQ",  bg: "bg-teal-500/20",    fg: "text-teal-300" };
    case "PosResponse":      return { label: "R+",   bg: "bg-emerald-500/20", fg: "text-emerald-300" };
    case "NegResponse":      return { label: "R-",   bg: "bg-rose-500/20",    fg: "text-rose-300" };
    case "FunctionalClass":  return { label: "FC",   bg: "bg-orange-500/20",  fg: "text-orange-300" };
    case "Sdg":              return { label: "SDG",  bg: "bg-lime-500/20",    fg: "text-lime-300" };
    case "Dop":              return { label: "DOP",  bg: "bg-pink-500/20",    fg: "text-pink-300" };
    case "ParentRefs":       return { label: "REF",  bg: "bg-cyan-500/20",    fg: "text-cyan-300" };
    default: break;
  }

  // Infer badge from text for DOP category children and service-list headers
  const t = node.text.toLowerCase();
  if (t.startsWith("diag-comms"))       return { label: "DC",  bg: "bg-violet-500/15", fg: "text-violet-300/70" };
  if (t.startsWith("requests"))          return { label: "REQ", bg: "bg-teal-500/15",   fg: "text-teal-300/70" };
  if (t.startsWith("pos-response") || t.startsWith("positive response"))
                                          return { label: "R+",  bg: "bg-emerald-500/15", fg: "text-emerald-300/70" };
  if (t.startsWith("neg-response") || t.startsWith("negative response"))
                                          return { label: "R-",  bg: "bg-rose-500/15",   fg: "text-rose-300/70" };
  if (t.startsWith("functional classes")) return { label: "FC",  bg: "bg-orange-500/15", fg: "text-orange-300/70" };
  if (t.startsWith("comparam"))          return { label: "CP",  bg: "bg-sky-500/15",    fg: "text-sky-300/70" };
  if (t.startsWith("state chart"))       return { label: "SC",  bg: "bg-indigo-500/15", fg: "text-indigo-300/70" };
  if (t.startsWith("sdgs"))             return { label: "SDG", bg: "bg-lime-500/15",   fg: "text-lime-300/70" };
  // DOP sub-categories
  if (t.startsWith("structures"))        return { label: "STR", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("data object"))       return { label: "DOP", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("dtc dop"))           return { label: "DTC", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("env data"))          return { label: "ENV", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("static field"))      return { label: "SF",  bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("dynamic"))           return { label: "DYN", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("end of pdu"))        return { label: "EOP", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };
  if (t.startsWith("mux"))              return { label: "MUX", bg: "bg-pink-500/15",   fg: "text-pink-300/70" };

  return null;
}

function diffBadge(status: string | null): Badge | null {
  switch (status) {
    case "Added":    return { label: "+",  bg: "bg-emerald-500/20", fg: "text-emerald-300" };
    case "Removed":  return { label: "-",  bg: "bg-red-500/20",     fg: "text-red-300" };
    case "Modified": return { label: "~",  bg: "bg-amber-500/20",   fg: "text-amber-300" };
    default:         return null;
  }
}

function nodeTextClass(node: VisibleNode): string {
  if (node.diff_status === "Removed") return "text-neutral-500 line-through";
  if (node.diff_status === "Unchanged") return "text-neutral-600";
  if (node.node_type === "SectionHeader") return "text-white font-semibold";
  if (node.node_type === "Container") return "text-neutral-100 font-medium";
  if (node.node_type === "ParentRefService") return "text-neutral-500 italic";
  if (node.node_type === "ParentRefs" || node.node_type === "Dop") return "text-neutral-200 font-medium";
  return "text-neutral-300";
}

const legendItems: Badge[] = [
  { label: "SVC", bg: "bg-violet-500/20", fg: "text-violet-300" },
  { label: "JOB", bg: "bg-violet-500/15", fg: "text-violet-300/70" },
  { label: "REQ", bg: "bg-teal-500/20",   fg: "text-teal-300" },
  { label: "R+",  bg: "bg-emerald-500/20", fg: "text-emerald-300" },
  { label: "R-",  bg: "bg-rose-500/20",   fg: "text-rose-300" },
  { label: "FC",  bg: "bg-orange-500/20", fg: "text-orange-300" },
  { label: "DOP", bg: "bg-pink-500/20",   fg: "text-pink-300" },
  { label: "SDG", bg: "bg-lime-500/20",   fg: "text-lime-300" },
  { label: "INH", bg: "bg-amber-500/15",   fg: "text-amber-400" },
  { label: "REF", bg: "bg-cyan-500/20",   fg: "text-cyan-300" },
];
const legendLabels: Record<string, string> = {
  SVC: "Service", JOB: "Job", REQ: "Request", "R+": "Pos-Response", "R-": "Neg-Response",
  FC: "Functional Class", DOP: "DOP / Structure", SDG: "Special Data Group",
  INH: "Inherited", REF: "Parent References",
};

async function onClick(node: VisibleNode) {
  await store.selectNode(node.index);
}
async function onDblClick(node: VisibleNode) {
  if (node.has_children) await store.toggleExpand(node.index);
}
async function onChevronClick(e: Event, node: VisibleNode) {
  e.stopPropagation();
  if (node.has_children) await store.toggleExpand(node.index);
}
</script>

<template>
  <div class="flex flex-col h-full bg-neutral-950">
    <!-- Header -->
    <div class="flex items-center h-8 px-2 border-b border-neutral-800/60 shrink-0 gap-1">
      <span class="text-[11px] text-neutral-500 font-medium uppercase tracking-wider flex-1">Explorer</span>
      <button class="p-1 rounded text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-colors" title="Search (/)" @click="store.searchActive = true">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
      </button>
      <button class="p-1 rounded transition-colors" :class="showLegend ? 'text-blue-400 bg-blue-500/10' : 'text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800'" title="Legend" @click="showLegend = !showLegend">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
      </button>
      <button class="p-1 rounded text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-colors" title="Sort (s)" @click="store.toggleSort()">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 16 4 4 4-4"/><path d="M7 20V4"/><path d="m21 8-4-4-4 4"/><path d="M17 4v16"/></svg>
      </button>
      <button class="p-1 rounded text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-colors" title="Expand all (e)" @click="store.expandAll()">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>
      </button>
      <button class="p-1 rounded text-neutral-600 hover:text-neutral-300 hover:bg-neutral-800 transition-colors" title="Collapse all (c)" @click="store.collapseAll()">
        <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 20 5-5 5 5"/><path d="m7 4 5 5 5-5"/></svg>
      </button>
    </div>

    <!-- Legend -->
    <div v-if="showLegend" class="border-b border-neutral-800/60 px-3 py-2.5 bg-neutral-900 shrink-0">
      <div class="text-[10px] text-neutral-500 uppercase tracking-wider font-medium mb-2">Badges</div>
      <div class="flex flex-wrap gap-2">
        <div v-for="b in legendItems" :key="b.label" class="flex items-center gap-1.5">
          <span class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none" :class="`${b.bg} ${b.fg}`">{{ b.label }}</span>
          <span class="text-[11px] text-neutral-500">{{ legendLabels[b.label] }}</span>
        </div>
      </div>
      <template v-if="store.isDiff">
        <div class="text-[10px] text-neutral-500 uppercase tracking-wider font-medium mt-2.5 mb-2">Diff</div>
        <div class="flex gap-3">
          <div class="flex items-center gap-1.5"><span class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none bg-emerald-500/20 text-emerald-300">+</span><span class="text-[11px] text-neutral-500">Added</span></div>
          <div class="flex items-center gap-1.5"><span class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none bg-red-500/20 text-red-300">-</span><span class="text-[11px] text-neutral-500">Removed</span></div>
          <div class="flex items-center gap-1.5"><span class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none bg-amber-500/20 text-amber-300">~</span><span class="text-[11px] text-neutral-500">Modified</span></div>
        </div>
      </template>
    </div>

    <!-- Node list -->
    <div class="flex-1 overflow-y-auto overflow-x-hidden py-0.5">
      <div v-if="store.nodes.length === 0" class="text-neutral-700 text-center text-xs mt-12">
        No nodes loaded
      </div>
      <div
        v-for="node in store.nodes"
        :key="node.index"
        class="flex items-center h-[24px] cursor-pointer transition-colors group gap-1"
        :class="node.index === store.selectedIndex
          ? 'bg-neutral-800'
          : 'hover:bg-neutral-900'"
        :style="{ paddingLeft: `${node.depth * 14 + 6}px` }"
        @click="onClick(node)"
        @dblclick="onDblClick(node)"
        @contextmenu="onContextMenu($event, node)"
      >
        <!-- Chevron -->
        <span
          v-if="node.has_children"
          class="w-4 h-4 flex items-center justify-center text-neutral-600 group-hover:text-neutral-400 shrink-0 transition-transform"
          :class="node.expanded ? 'rotate-0' : '-rotate-90'"
          @click="onChevronClick($event, node)"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><path d="m7 10 5 5 5-5z"/></svg>
        </span>
        <span v-else class="w-4 shrink-0" />

        <!-- Diff badge -->
        <span
          v-if="diffBadge(node.diff_status)"
          class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-bold leading-none shrink-0"
          :class="`${diffBadge(node.diff_status)!.bg} ${diffBadge(node.diff_status)!.fg}`"
        >{{ diffBadge(node.diff_status)!.label }}</span>

        <!-- Type badge -->
        <span
          v-if="nodeBadge(node)"
          class="inline-flex items-center justify-center rounded px-1 py-px text-[9px] font-semibold leading-none shrink-0"
          :class="`${nodeBadge(node)!.bg} ${nodeBadge(node)!.fg}`"
        >{{ nodeBadge(node)!.label }}</span>

        <!-- Label -->
        <span class="truncate text-[12px] leading-tight" :class="nodeTextClass(node)">{{ node.text }}</span>
      </div>
    </div>

    <!-- Context menu -->
    <Teleport to="body">
      <div
        v-if="ctxMenu"
        class="fixed z-50 min-w-44 py-1 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl shadow-black/40 text-[12px]"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
      >
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="ctxAction('select')">
          View details
        </button>
        <button
          v-if="ctxMenu.node.has_children"
          class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors"
          @click="ctxAction('expand')"
        >
          {{ ctxMenu.node.expanded ? 'Collapse' : 'Expand' }}
        </button>
        <div class="h-px bg-neutral-800 my-1" />
        <button
          v-if="ctxMenu.node.node_type === 'ParentRefService'"
          class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors"
          @click="ctxAction('goToParent')"
        >
          Go to parent definition
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="ctxAction('copyName')">
          Copy name
        </button>
        <div class="h-px bg-neutral-800 my-1" />
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="ctxAction('sort')">
          Sort tree
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="ctxAction('expandAll')">
          Expand all
        </button>
        <button class="w-full text-left px-3 py-1.5 text-neutral-300 hover:bg-neutral-800 transition-colors" @click="ctxAction('collapseAll')">
          Collapse all
        </button>
      </div>
    </Teleport>
  </div>
</template>
