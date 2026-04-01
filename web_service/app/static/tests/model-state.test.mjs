import test from 'node:test';
import assert from 'node:assert/strict';
import {
  DescriptionHistory,
  clearIsolation,
  collectObjectStateMap,
  createStepLikeExport,
  groupShapes,
  hideShapes,
  isolateShapes,
  lockShapes,
  normalizeDescription,
  showAllShapes,
  ungroupShapes,
  unlockShapes,
} from '../model-state.mjs';

function seedDescription() {
  return normalizeDescription({
    name: 'demo',
    unit: 'mm',
    precision: 0.01,
    shapes: [
      { id: 'a', type: 'box', position: [0, 0, 0], dimensions: { width: 1, height: 1, depth: 1 } },
      { id: 'b', type: 'box', position: [1, 0, 0], dimensions: { width: 1, height: 1, depth: 1 } },
      { id: 'c', type: 'sphere', position: [2, 0, 0], dimensions: { radius: 1 } },
    ],
    features: [],
    constraints: [],
  });
}

test('history undo/redo roundtrip', () => {
  const history = new DescriptionHistory(10);
  const base = seedDescription();
  const edited = seedDescription();
  edited.shapes[0].position = [5, 0, 0];

  history.push(base, 'base');
  const undone = history.undo(edited);
  assert.deepEqual(undone.shapes[0].position, [0, 0, 0]);

  const redone = history.redo(undone);
  assert.deepEqual(redone.shapes[0].position, [5, 0, 0]);
});

test('group and ungroup mutate selected shapes only', () => {
  const grouped = groupShapes(seedDescription(), ['a', 'b']);
  assert.ok(grouped.shapes[0].group_id);
  assert.equal(grouped.shapes[0].group_id, grouped.shapes[1].group_id);
  assert.equal(grouped.shapes[2].group_id, undefined);

  const ungrouped = ungroupShapes(grouped, ['a']);
  assert.equal(ungrouped.shapes[0].group_id, undefined);
  assert.ok(ungrouped.shapes[1].group_id);
});

test('visibility + isolation flags resolve into object states', () => {
  const hidden = hideShapes(seedDescription(), ['a']);
  const isolated = isolateShapes(hidden, ['b']);
  const states = collectObjectStateMap(isolated);
  assert.equal(states.get('a').hidden, true);
  assert.equal(states.get('b').isolate, true);

  const cleared = clearIsolation(isolated);
  const shown = showAllShapes(cleared);
  const reset = collectObjectStateMap(shown);
  assert.equal(reset.get('a').hidden, false);
  assert.equal(reset.get('b').isolate, false);
});

test('lock and unlock shapes', () => {
  const locked = lockShapes(seedDescription(), ['a', 'c']);
  assert.equal(locked.shapes[0].locked, true);
  assert.equal(locked.shapes[1].locked, undefined);
  assert.equal(locked.shapes[2].locked, true);

  const unlocked = unlockShapes(locked, ['a']);
  assert.equal(unlocked.shapes[0].locked, undefined);
  assert.equal(unlocked.shapes[2].locked, true);
});

test('step-like export emits standard envelope', () => {
  const text = createStepLikeExport(seedDescription());
  assert.match(text, /ISO-10303-21;/);
  assert.match(text, /END-ISO-10303-21;/);
  assert.match(text, /SHAPE a/);
});
