import assert from 'node:assert/strict';
import { setImmediate } from 'node:timers/promises';
import test from 'node:test';
import { QueryClient, QueryObserver, InfiniteQueryObserver } from '@tanstack/react-query';
import { refreshLibraryQueries, subscribeLibraryChanges } from '../src/hooks/libraryEvents.ts';

function clientFor(t) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: Infinity, staleTime: Infinity } } });
  t.after(() => client.clear());
  return client;
}
function observe(t, client, queryKey, queryFn) {
  const observer = new QueryObserver(client, { queryKey, queryFn, refetchOnMount: false });
  t.after(observer.subscribe(() => {}));
}
function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function eventSource(t, client, errors = []) {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  let emit;
  let cleanups = 0;
  const dispose = subscribeLibraryChanges(client, async (callback) => {
    emit = callback;
    return () => { cleanups++; };
  }, (error) => errors.push(error));
  t.after(dispose);
  return { emit: (ids) => emit(ids), dispose, cleanups: () => cleanups };
}
async function tick(t) {
  t.mock.timers.tick(100);
  await setImmediate();
}

test('a previously absent track can enter search results after a library change', async (t) => {
  const client = clientFor(t);
  const key = ['searchMedia', 'songs', 'new title'];
  client.setQueryData(key, { pages: [{ files: [], next_cursor: null }], pageParams: [0] });
  observe(t, client, key, async () => ({ pages: [{ files: [{ id: 42 }], next_cursor: null }], pageParams: [0] }));
  await refreshLibraryQueries(client, new Set([42]));
  assert.equal(client.getQueryData(key).pages[0].files[0].id, 42);
});

test('inactive lists and aggregates become stale without background fetching', async (t) => {
  const client = clientFor(t);
  const keys = [['fileWatcherMap'], ['libraryCollections', 'albums'], ['homeDiscovery', 'all'], ['playlist', 1], ['playlists'], ['favoriteIds'], ['missingFiles'], ['rewind', 2026, null]];
  for (const key of keys) client.setQueryData(key, []);
  await refreshLibraryQueries(client, new Set([42]));
  for (const key of keys) {
    assert.equal(client.getQueryState(key).isInvalidated, true, JSON.stringify(key));
    assert.equal(client.getQueryState(key).fetchStatus, 'idle');
  }
});

test('only affected media and artwork caches are invalidated', async (t) => {
  const client = clientFor(t);
  const affected = [['mediaFiles', [1, 2]], ['libraryArtwork', 2], ['artworkDetails', [2, 3]]];
  const unaffected = [['mediaFiles', [3]], ['libraryArtwork', 3], ['artworkDetails', [3]], ['unrelated']];
  for (const key of [...affected, ...unaffected]) client.setQueryData(key, []);
  await refreshLibraryQueries(client, new Set([2]));
  for (const key of affected) assert.equal(client.getQueryState(key).isInvalidated, true);
  for (const key of unaffected) assert.equal(client.getQueryState(key).isInvalidated, false);
});

test('an old initial request cannot overwrite a refresh', async (t) => {
  const client = clientFor(t), old = deferred(), key = ['mediaFiles', [1]];
  let calls = 0;
  observe(t, client, key, () => ++calls === 1 ? old.promise : Promise.resolve('new path'));
  assert.equal(calls, 1);
  await refreshLibraryQueries(client, new Set([1]));
  old.resolve('old path'); await setImmediate();
  assert.equal(calls, 2);
  assert.equal(client.getQueryData(key), 'new path');
});

test('burst events produce one refresh', async (t) => {
  const client = clientFor(t), key = ['mediaFiles', [1, 2]];
  let calls = 0;
  client.setQueryData(key, 'before');
  observe(t, client, key, async () => { calls++; return 'after'; });
  const source = eventSource(t, client);
  source.emit([1]); source.emit([1, 2]); source.emit([2]);
  assert.equal(calls, 0);
  await tick(t);
  assert.equal(calls, 1);
  assert.equal(client.getQueryData(key), 'after');
});

for (const fails of [false, true]) {
  test(`events during ${fails ? 'a failed' : 'an ongoing'} refresh run in a later batch`, async (t) => {
    const client = clientFor(t), first = deferred(), errors = [], key = ['fileWatcherMap'];
    let calls = 0;
    client.setQueryData(key, []);
    observe(t, client, key, () => ++calls === 1 ? first.promise : Promise.resolve(['latest']));
    const source = eventSource(t, client, errors);
    source.emit([1]); await tick(t);
    source.emit([2]); await tick(t);
    assert.equal(calls, 1);
    if (fails) first.reject(new Error('temporary failure'));
    else first.resolve(['intermediate']);
    await setImmediate(); await tick(t);
    assert.equal(calls, 2);
    assert.equal(errors.length, fails ? 1 : 0);
    assert.deepEqual(client.getQueryData(key), ['latest']);
  });
}

test('cleanup cancels pending work and ignores late events', async (t) => {
  const client = clientFor(t), key = ['fileWatcherMap'];
  client.setQueryData(key, []);
  const source = eventSource(t, client);
  await setImmediate();
  source.emit([1]); source.dispose(); source.emit([2]);
  await tick(t);
  assert.equal(client.getQueryState(key).isInvalidated, false);
  assert.equal(source.cleanups(), 1);
});

test('a listener resolving after unmount is immediately removed', async (t) => {
  const client = clientFor(t), registration = deferred();
  let cleanups = 0;
  const dispose = subscribeLibraryChanges(client, () => registration.promise, assert.fail);
  dispose(); registration.resolve(() => { cleanups++; });
  await setImmediate();
  assert.equal(cleanups, 1);
});

test('listener registration failures are handled', async (t) => {
  const client = clientFor(t), errors = [];
  const dispose = subscribeLibraryChanges(client, async () => { throw new Error('listener unavailable'); }, (e) => errors.push(e));
  t.after(dispose);
  await setImmediate();
  assert.equal(errors[0].message, 'listener unavailable');
});


test('paginated library refresh removes missing tracks and fills pages with restored tracks', async (t) => {
  const client = clientFor(t), key = ['fileWatcherMap'];
  client.setQueryData(key, {
    pages: [{ files: [1, 2], next_cursor: 2 }, { files: [3, 4], next_cursor: null }],
    pageParams: [0, 2],
  });
  const available = [1, 3, 4, 5];
  const observer = new InfiniteQueryObserver(client, {
    queryKey: key,
    initialPageParam: 0,
    refetchOnMount: false,
    queryFn: async ({ pageParam }) => ({
      files: available.slice(pageParam, pageParam + 2),
      next_cursor: pageParam + 2 < available.length ? pageParam + 2 : null,
    }),
    getNextPageParam: (page) => page.next_cursor ?? undefined,
  });
  t.after(observer.subscribe(() => {}));
  await refreshLibraryQueries(client, new Set([2, 5]));
  assert.deepEqual(client.getQueryData(key).pages.flatMap((page) => page.files), available);
  assert.deepEqual(client.getQueryData(key).pageParams, [0, 2]);
});

test('disposing during a refresh discards queued follow-up work', async (t) => {
  const client = clientFor(t), first = deferred(), key = ['fileWatcherMap'];
  let calls = 0;
  client.setQueryData(key, []);
  observe(t, client, key, () => { calls++; return first.promise; });
  const source = eventSource(t, client);
  source.emit([1]); await tick(t);
  source.emit([2]); source.dispose(); source.dispose();
  first.resolve([]); await setImmediate(); await tick(t);
  assert.equal(calls, 1);
  assert.equal(source.cleanups(), 1);
});
