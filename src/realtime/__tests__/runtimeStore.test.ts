import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { applyRealtimeEvent, clearRuntimeStore, getRoomMessages, seedRoomMessages } from '../runtimeStore';
import type { MessageInfo } from '../../types';

function createMessage(overrides: Partial<MessageInfo>): MessageInfo {
  return {
    db_id: 1,
    sender_id: -1,
    sender_display_name: 'OPERATOR',
    content: 'message',
    time: '2026-05-04 22:24:00',
    seq: null,
    insert_immediately: false,
    ...overrides,
  };
}

describe('runtimeStore message ordering', () => {
  beforeEach(() => {
    clearRuntimeStore();
  });

  afterEach(() => {
    clearRuntimeStore();
  });

  it('re-sorts messages when a queued message receives seq via message_changed', () => {
    seedRoomMessages(3, [
      createMessage({ db_id: 27, content: '测试27', seq: 0, time: '2026-05-04 22:24:29' }),
      createMessage({ db_id: 28, content: '测试28', seq: null, time: '2026-05-04 22:24:41' }),
    ]);

    applyRealtimeEvent({
      type: 'message_changed',
      teamId: 2,
      roomId: 3,
      roomName: '小马哥',
      message: createMessage({ db_id: 28, content: '测试28', seq: 1, time: '2026-05-04 22:24:41' }),
    });

    const messages = getRoomMessages(3);
    expect(messages.map((message) => message.db_id)).toEqual([27, 28]);
    expect(messages.map((message) => message.seq)).toEqual([0, 1]);
  });
});
