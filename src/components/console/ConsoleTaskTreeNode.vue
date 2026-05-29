<script setup lang="ts">
import { computed } from 'vue';
import AgentTaskCard from '../agent/AgentTaskCard.vue';
import type { TaskTreeNode } from '../../utils/taskTree';

defineOptions({
  name: 'ConsoleTaskTreeNode',
});

const props = defineProps<{
  node: TaskTreeNode;
  isChild?: boolean;
  agentLabels: Map<number, string>;
}>();

const emit = defineEmits<{
  selectTask: [taskId: number];
}>();

const assigneeLabel = computed(() => {
  const assigneeId = props.node.task.assignee_id;
  return props.agentLabels.get(assigneeId) || `#${assigneeId}`;
});

const managerLabel = computed(() => {
  const managerId = props.node.task.manager_id;
  if (typeof managerId !== 'number' || managerId <= 0) {
    return null;
  }
  return props.agentLabels.get(managerId) || `#${managerId}`;
});

function handleSelect(): void {
  emit('selectTask', props.node.task.id);
}
</script>

<template>
  <div
    class="task-branch"
    :class="{
      'task-branch--child': isChild,
      'task-branch--has-children': node.children.length > 0,
    }"
  >
    <AgentTaskCard
      class="task-card"
      :task="node.task"
      :assignee-label="assigneeLabel"
      :manager-label="managerLabel"
      clickable
      variant="tree"
      @select="handleSelect"
    />

    <div v-if="node.children.length" class="task-branch__children">
      <ConsoleTaskTreeNode
        v-for="child in node.children"
        :key="child.task.id"
        :node="child"
        :is-child="true"
        :agent-labels="agentLabels"
        @select-task="emit('selectTask', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.task-branch {
  --task-tree-link-gap: 20px;
  --task-tree-link-indent: 20px;
  --task-tree-card-center: 24px;
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--task-tree-link-gap);
  min-width: max-content;
}

.task-branch--has-children > .task-card {
  position: relative;
}

.task-branch--has-children > .task-card::after {
  content: '';
  position: absolute;
  top: 50%;
  right: calc(var(--task-tree-link-gap) * -1);
  width: var(--task-tree-link-gap);
  border-top: 1px solid color-mix(in srgb, var(--panel-border-strong) 74%, transparent);
  transform: translateY(-0.5px);
}

.task-branch--child::before {
  content: '';
  position: absolute;
  left: calc(var(--task-tree-link-indent) * -1);
  top: 50%;
  width: var(--task-tree-link-indent);
  border-top: 1px solid color-mix(in srgb, var(--panel-border-strong) 78%, transparent);
  transform: translateY(-0.5px);
}

.task-branch__children {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-left: var(--task-tree-link-indent);
}

.task-branch__children::before {
  content: '';
  position: absolute;
  left: 0;
  top: var(--task-tree-card-center);
  bottom: var(--task-tree-card-center);
  border-left: 1px solid color-mix(in srgb, var(--panel-border-strong) 74%, transparent);
}

</style>
