<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/app";
import type { DetailSection, DetailContent, JumpTarget } from "../api/commands";

const store = useAppStore();

const headerSection = computed<DetailSection | null>(() => {
  const s = store.detailSections;
  if (s.length > 1 && s[0].render_as_header && "PlainText" in s[0].content) return s[0];
  return null;
});

const tabSections = computed<DetailSection[]>(() =>
  headerSection.value ? store.detailSections.slice(1) : store.detailSections,
);

const activeSection = computed<DetailSection | null>(() =>
  tabSections.value[store.selectedTab] ?? null,
);

function text(c: DetailContent): string[] | null { return "PlainText" in c ? c.PlainText : null; }
function table(c: DetailContent) { return "Table" in c ? c.Table : null; }
function composite(c: DetailContent): DetailSection[] | null { return "Composite" in c ? c.Composite : null; }

function diffCls(s: string | null): string {
  if (s === "Added") return "bg-emerald-500/5 border-l-2 border-l-emerald-500/40";
  if (s === "Removed") return "bg-red-500/5 border-l-2 border-l-red-500/30 opacity-60 line-through";
  if (s === "Modified") return "bg-amber-500/5 border-l-2 border-l-amber-500/30";
  return "";
}

async function nav(t: JumpTarget | null) { if (t) await store.navigateTo(t); }
</script>

<template>
  <div class="flex flex-col h-full bg-[#0e1018]">
    <!-- Empty state -->
    <div v-if="!store.selectedNode" class="flex-1 flex items-center justify-center">
      <div class="text-center text-gray-700 text-sm">
        <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" class="mx-auto mb-3 text-gray-800"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m16 15-3-3 3-3"/></svg>
        Select a node
      </div>
    </div>

    <template v-else>
      <!-- Header info -->
      <div
        v-if="headerSection && text(headerSection.content)"
        class="px-4 py-2.5 border-b border-gray-800/60 bg-[#0c0e14]"
      >
        <div
          v-for="(line, i) in text(headerSection.content)"
          :key="i"
          class="text-[12px] text-gray-500 leading-relaxed"
        >{{ line }}</div>
      </div>

      <!-- Tabs -->
      <div v-if="tabSections.length > 1" class="flex border-b border-gray-800/60 bg-[#0c0e14] overflow-x-auto shrink-0">
        <button
          v-for="(section, i) in tabSections"
          :key="i"
          class="px-4 py-2 text-[12px] whitespace-nowrap border-b-2 transition-colors"
          :class="i === store.selectedTab
            ? 'border-blue-500 text-gray-200'
            : 'border-transparent text-gray-600 hover:text-gray-400'"
          @click="store.selectedTab = i"
        >{{ section.title }}</button>
      </div>

      <!-- Content -->
      <div v-if="activeSection" class="flex-1 overflow-auto">
        <!-- Plain text -->
        <div v-if="text(activeSection.content)" class="p-4 space-y-1">
          <p
            v-for="(line, i) in text(activeSection.content)"
            :key="i"
            class="text-[12px] text-gray-400 leading-relaxed"
          >{{ line || "\u00A0" }}</p>
        </div>

        <!-- Table -->
        <div v-else-if="table(activeSection.content)" class="overflow-x-auto">
          <table class="w-full text-[12px]">
            <thead class="sticky top-0 bg-[#0e1018] z-10">
              <tr class="border-b border-gray-800/80">
                <th
                  v-for="(cell, ci) in table(activeSection.content)!.header.cells"
                  :key="ci"
                  class="text-left px-3 py-2 text-[11px] text-gray-500 font-medium uppercase tracking-wider"
                >{{ cell.text }}</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, ri) in table(activeSection.content)!.rows"
                :key="ri"
                class="border-b border-gray-800/30 hover:bg-gray-800/20 transition-colors"
                :class="diffCls(row.diff_status)"
              >
                <td
                  v-for="(cell, ci) in row.cells"
                  :key="ci"
                  class="px-3 py-1.5 text-gray-400"
                  :class="{
                    'text-blue-400 cursor-pointer hover:text-blue-300 hover:underline': cell.jump_target,
                    'text-gray-300 font-medium': cell.cell_type === 'ParameterName',
                  }"
                  :style="ci === 0 && row.indent > 0 ? { paddingLeft: `${row.indent * 10 + 12}px` } : {}"
                  @click="nav(cell.jump_target)"
                >{{ cell.text }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Composite -->
        <div v-else-if="composite(activeSection.content)" class="p-3 space-y-3">
          <div
            v-for="(sub, si) in composite(activeSection.content)"
            :key="si"
            class="rounded-lg border border-gray-800/50 overflow-hidden"
          >
            <div class="px-3 py-1.5 bg-gray-800/20 text-[11px] text-gray-500 font-medium uppercase tracking-wider">
              {{ sub.title }}
            </div>
            <div v-if="text(sub.content)" class="px-3 py-2">
              <p
                v-for="(line, li) in text(sub.content)"
                :key="li"
                class="text-[12px] text-gray-400"
              >{{ line || "\u00A0" }}</p>
            </div>
            <table v-else-if="table(sub.content)" class="w-full text-[12px]">
              <thead>
                <tr class="border-b border-gray-800/50">
                  <th
                    v-for="(cell, ci) in table(sub.content)!.header.cells"
                    :key="ci"
                    class="text-left px-3 py-1.5 text-[11px] text-gray-600 font-medium"
                  >{{ cell.text }}</th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, ri) in table(sub.content)!.rows"
                  :key="ri"
                  class="border-b border-gray-800/20"
                  :class="diffCls(row.diff_status)"
                >
                  <td
                    v-for="(cell, ci) in row.cells"
                    :key="ci"
                    class="px-3 py-1 text-gray-400"
                    :class="{ 'text-blue-400 cursor-pointer hover:underline': cell.jump_target }"
                    @click="nav(cell.jump_target)"
                  >{{ cell.text }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <div v-else class="flex-1 flex items-center justify-center text-gray-700 text-xs">
        No details available
      </div>
    </template>
  </div>
</template>
