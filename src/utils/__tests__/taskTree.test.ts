import { describe, expect, it } from 'vitest';
import type { AgentTask } from '../../types';
import { buildTaskForest } from '../taskTree';

function buildTask(overrides: Partial<AgentTask> & Pick<AgentTask, 'id' | 'title'>): AgentTask {
  return {
    id: overrides.id,
    team_id: overrides.team_id ?? 1,
    title: overrides.title,
    description: overrides.description ?? '',
    assignee_id: overrides.assignee_id ?? 1,
    creator_id: overrides.creator_id ?? 1,
    manager_id: overrides.manager_id ?? null,
    status: overrides.status ?? 'TODO',
    priority: overrides.priority ?? 'NORMAL',
    parent_id: overrides.parent_id ?? null,
    depends_on: overrides.depends_on ?? [],
    room_id: overrides.room_id ?? null,
    result: overrides.result ?? '',
    block_reason: overrides.block_reason ?? '',
    created_at: overrides.created_at ?? null,
    updated_at: overrides.updated_at ?? null,
  };
}

describe('buildTaskForest', () => {
  it('builds parent-child trees and keeps orphan tasks as roots', () => {
    const forest = buildTaskForest([
      buildTask({ id: 3, title: 'Child A-1', parent_id: 1 }),
      buildTask({ id: 1, title: 'Root A' }),
      buildTask({ id: 5, title: 'Orphan child', parent_id: 99 }),
      buildTask({ id: 2, title: 'Root B' }),
      buildTask({ id: 4, title: 'Child A-2', parent_id: 1 }),
    ]);

    expect(forest.map((node) => node.task.id)).toEqual([1, 2, 5]);
    expect(forest[0]?.children.map((node) => node.task.id)).toEqual([3, 4]);
    expect(forest[0]?.depth).toBe(0);
    expect(forest[0]?.children[0]?.depth).toBe(1);
  });

  it('breaks self-parent cycles by promoting the task to a root', () => {
    const forest = buildTaskForest([
      buildTask({ id: 7, title: 'Loop', parent_id: 7 }),
    ]);

    expect(forest).toHaveLength(1);
    expect(forest[0]?.task.id).toBe(7);
    expect(forest[0]?.children).toEqual([]);
  });
});
