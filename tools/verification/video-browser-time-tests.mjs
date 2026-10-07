import assert from 'node:assert/strict';
import { test } from 'node:test';
import { compare, contains, fromSeconds, interval, lookupSeconds } from '../../.codex-work/video-browser-media/build/video-browser-media/src/time.js';
const time = (n, d = 1, side = 'after') => ({ time: { numerator: String(n), denominator: String(d) }, side });

test('one-sided source sampling uses exact container ticks including negative times', () => {
  assert.equal(lookupSeconds(time(1001, 30000), 30000), 1001 / 30000);
  assert.equal(lookupSeconds(time(1001, 30000, 'before'), 30000), 1000 / 30000);
  assert.equal(lookupSeconds(time(2003, 60000, 'before'), 30000), 1001 / 30000);
  assert.equal(lookupSeconds(time(-1, 60000), 30000), -1 / 30000);
  assert.equal(lookupSeconds(time(-1, 30000, 'before'), 30000), -2 / 30000);
  assert.equal(lookupSeconds(time(0, 1, 'before'), 30000), -1 / 30000);
});
test('half-open forward and reverse boundaries never stretch a sample into a gap', () => {
  const span = interval(1001 / 30000, 1001 / 30000, 30000);
  assert.equal(contains(span, time(1001, 30000)), true);
  assert.equal(contains(span, time(1001, 30000, 'before')), false);
  assert.equal(contains(span, time(2002, 30000)), false);
  assert.equal(contains(span, time(2002, 30000, 'before')), true);
  assert.equal(contains(span, time(5)), false);
});
test('fractional ticks are recovered independently, without cumulative microsecond loss', () => {
  for (let frame = 0; frame < 107892; frame++) {
    const ticks = frame * 1001;
    assert.equal(fromSeconds(ticks / 30000, 30000).ticks, String(ticks));
  }
  assert.equal(compare(time(1, 3).time, time(333333, 1000000).time), 1);
});
test('reject invalid, unsafe and unbounded timing instead of approximate fallback', () => {
  for (const n of ['-0', '+1', '01', '1.0', '1e6', '9'.repeat(1236)])
    assert.throws(() => lookupSeconds(time(n), 30000));
  assert.throws(() => lookupSeconds(time(1, 0), 30000));
  assert.throws(() => lookupSeconds(time('9'.repeat(100)), 30000));
  assert.throws(() => fromSeconds(0.0001234, 1000));
  assert.throws(() => interval(1, 0, 30000));
});
