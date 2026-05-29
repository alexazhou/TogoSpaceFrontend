import type { AgentTask } from '../types';

export interface TaskTreeNode {
  task: AgentTask;
  children: TaskTreeNode[];
  depth: number;
}

function sortTasksById(tasks: AgentTask[]): AgentTask[] {
  return [...tasks].sort((left, right) => left.id - right.id);
}

export function buildTaskForest(tasks: AgentTask[]): TaskTreeNode[] {
  const sortedTasks = sortTasksById(tasks);
  const nodeMap = new Map<number, TaskTreeNode>(
    sortedTasks.map((task) => [task.id, { task, children: [], depth: 0 }]),
  );
  const roots: TaskTreeNode[] = [];

  for (const task of sortedTasks) {
    const node = nodeMap.get(task.id);
    if (!node) {
      continue;
    }

    const parent = task.parent_id !== null ? nodeMap.get(task.parent_id) : null;
    if (!parent || parent === node) {
      roots.push(node);
      continue;
    }

    parent.children.push(node);
  }

  const visited = new Set<number>();

  function assignDepth(node: TaskTreeNode, depth: number, path: Set<number>): void {
    if (visited.has(node.task.id)) {
      return;
    }

    visited.add(node.task.id);
    node.depth = depth;

    const nextPath = new Set(path);
    nextPath.add(node.task.id);
    node.children = node.children.filter((child) => !nextPath.has(child.task.id));
    for (const child of node.children) {
      assignDepth(child, depth + 1, nextPath);
    }
  }

  for (const root of roots) {
    assignDepth(root, 0, new Set());
  }

  for (const task of sortedTasks) {
    if (visited.has(task.id)) {
      continue;
    }

    const node = nodeMap.get(task.id);
    if (!node) {
      continue;
    }

    roots.push(node);
    assignDepth(node, 0, new Set());
  }

  return roots;
}
