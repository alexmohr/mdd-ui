<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/app";
import type { DetailSection, DetailContent, JumpTarget } from "../api/commands";

const store = useAppStore();

const headerSection = computed<DetailSection | null>(() => {
  const sections = store.detailSections;
  if (
    sections.length > 1 &&
    sections[0].render_as_header &&
    "PlainText" in sections[0].content
  ) {
    return sections[0];
  }
  return null;
});

const tabSections = computed<DetailSection[]>(() => {
  if (headerSection.value) {
    return store.detailSections.slice(1);
  }
  return store.detailSections;
});

const activeSection = computed<DetailSection | null>(() => {
  return tabSections.value[store.selectedTab] ?? null;
});

function getPlainText(content: DetailContent): string[] | null {
  if ("PlainText" in content) return content.PlainText;
  return null;
}

function getTable(content: DetailContent) {
  if ("Table" in content) return content.Table;
  return null;
}

function getComposite(content: DetailContent): DetailSection[] | null {
  if ("Composite" in content) return content.Composite;
  return null;
}

function diffRowClass(status: string | null): string {
  if (status === "Added") return "bg-green-950/30";
  if (status === "Removed") return "bg-red-950/30 line-through";
  if (status === "Modified") return "bg-yellow-950/20";
  return "";
}

async function onCellClick(target: JumpTarget | null) {
  if (target) {
    await store.navigateTo(target);
  }
}
</script>

<template>
  <div class="flex flex-col h-full">
    <div class="px-2 py-1 bg-gray-900 border-b border-gray-800 text-xs text-gray-500 font-semibold uppercase tracking-wide">
      Details
    </div>

    <div v-if="!store.selectedNode" class="flex-1 flex items-center justify-center text-gray-600">
      Select a node to view details
    </div>

    <template v-else>
      <!-- Header section (rendered above tabs) -->
      <div
        v-if="headerSection && getPlainText(headerSection.content)"
        class="px-3 py-2 bg-gray-900/50 border-b border-gray-800 text-xs"
      >
        <div v-for="(line, i) in getPlainText(headerSection.content)" :key="i" class="text-gray-400">
          {{ line }}
        </div>
      </div>

      <!-- Tab bar -->
      <div v-if="tabSections.length > 1" class="flex bg-gray-900 border-b border-gray-800 overflow-x-auto">
        <button
          v-for="(section, i) in tabSections"
          :key="i"
          class="px-3 py-1 text-xs whitespace-nowrap border-b-2 transition-colors"
          :class="
            i === store.selectedTab
              ? 'border-blue-500 text-blue-400 bg-gray-800/50'
              : 'border-transparent text-gray-500 hover:text-gray-300 hover:bg-gray-800/30'
          "
          @click="store.selectedTab = i"
        >
          {{ section.title }}
        </button>
      </div>

      <!-- Active section content -->
      <div v-if="activeSection" class="flex-1 overflow-auto p-2">
        <!-- Plain text -->
        <template v-if="getPlainText(activeSection.content)">
          <div class="space-y-0.5">
            <div
              v-for="(line, i) in getPlainText(activeSection.content)"
              :key="i"
              class="text-gray-300 text-xs"
            >
              {{ line || "\u00A0" }}
            </div>
          </div>
        </template>

        <!-- Table -->
        <template v-else-if="getTable(activeSection.content)">
          <div class="overflow-x-auto">
            <table class="w-full text-xs">
              <thead>
                <tr class="border-b border-gray-700">
                  <th
                    v-for="(cell, ci) in getTable(activeSection.content)!.header.cells"
                    :key="ci"
                    class="text-left px-2 py-1 text-gray-400 font-semibold whitespace-nowrap"
                  >
                    {{ cell.text }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="(row, ri) in getTable(activeSection.content)!.rows"
                  :key="ri"
                  class="border-b border-gray-800/50 hover:bg-gray-800/30"
                  :class="diffRowClass(row.diff_status)"
                >
                  <td
                    v-for="(cell, ci) in row.cells"
                    :key="ci"
                    class="px-2 py-0.5 text-gray-300 whitespace-nowrap"
                    :class="{
                      'text-blue-400 cursor-pointer hover:underline': cell.jump_target,
                      'font-semibold': cell.cell_type === 'ParameterName',
                    }"
                    :style="ci === 0 ? { paddingLeft: `${row.indent * 8 + 8}px` } : {}"
                    @click="onCellClick(cell.jump_target)"
                  >
                    {{ cell.text }}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>

        <!-- Composite -->
        <template v-else-if="getComposite(activeSection.content)">
          <div class="space-y-3">
            <div
              v-for="(sub, si) in getComposite(activeSection.content)"
              :key="si"
              class="border border-gray-800 rounded"
            >
              <div class="px-2 py-1 bg-gray-900 text-gray-400 text-xs font-semibold border-b border-gray-800">
                {{ sub.title }}
              </div>
              <div class="p-2">
                <template v-if="getPlainText(sub.content)">
                  <div
                    v-for="(line, li) in getPlainText(sub.content)"
                    :key="li"
                    class="text-gray-300 text-xs"
                  >
                    {{ line || "\u00A0" }}
                  </div>
                </template>
                <template v-else-if="getTable(sub.content)">
                  <table class="w-full text-xs">
                    <thead>
                      <tr class="border-b border-gray-700">
                        <th
                          v-for="(cell, ci) in getTable(sub.content)!.header.cells"
                          :key="ci"
                          class="text-left px-2 py-0.5 text-gray-400 font-semibold"
                        >
                          {{ cell.text }}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="(row, ri) in getTable(sub.content)!.rows"
                        :key="ri"
                        class="border-b border-gray-800/50"
                        :class="diffRowClass(row.diff_status)"
                      >
                        <td
                          v-for="(cell, ci) in row.cells"
                          :key="ci"
                          class="px-2 py-0.5 text-gray-300"
                          :class="{
                            'text-blue-400 cursor-pointer hover:underline': cell.jump_target,
                          }"
                          @click="onCellClick(cell.jump_target)"
                        >
                          {{ cell.text }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </template>
              </div>
            </div>
          </div>
        </template>
      </div>

      <div v-else class="flex-1 flex items-center justify-center text-gray-600 text-xs">
        No detail sections for this node
      </div>
    </template>
  </div>
</template>
