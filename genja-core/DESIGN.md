# Genja Core Design Notes

## Connection Manager Lifecycle Locking

`ConnectionManager` stores two per-connection-key structures:

- `connections_map: DashMap<ConnectionKey, Arc<Mutex<dyn Connection>>>`
- `connection_locks: DashMap<ConnectionKey, Arc<Mutex<()>>>`

The connection map stores the actual cached connection object. The mutex inside
that value protects mutable plugin state for a specific connection instance, such
as `open`, `execute_command`, and `close`.

The lifecycle lock map stores one async mutex per `ConnectionKey`. Its value is
`Arc<Mutex<()>>` because it carries no data; it is only a stable gate for cache
lifecycle operations. A lifecycle operation is any async path that may create,
open, evict, replace, or insert the cached connection for a key.

These locks are separate because the connection object mutex cannot protect the
cache slot while the entry is absent. During replacement, the old connection must
be removed before the new connection is created and opened. Without a stable
per-key lifecycle lock, another opener could observe the missing cache entry,
create a competing connection, and insert or return it while replacement is still
in progress.

Protected async methods use this workflow:

```text
open_connection(key)
    lock connection_locks[key]
    get or create connections_map[key]
    open the connection if it is not alive
    unlock connection_locks[key]

replace_connection(key)
    lock connection_locks[key]
    remove connections_map[key]
    close old connection best-effort
    create a new connection through the factory
    open the new connection
    insert connections_map[key]
    unlock connection_locks[key]
```

The lifecycle lock is held across async `open` and `close` calls on purpose. It
serializes ownership of the cache slot for the key. Different keys use different
mutexes and can still make progress concurrently.

`connection_locks` entries are intentionally not removed during normal lifecycle
operations. Removing a lock entry while an operation still holds its `Arc` could
allow another operation to create a second lock for the same key, breaking
serialization. The memory cost is one small lock per touched connection key for
the lifetime of the manager.

`get_or_create` remains a low-level synchronous helper for existing callers and
tests. New async lifecycle methods should acquire `lock_connection_lifecycle`
first and use the private `get_or_create_unlocked` helper while the lifecycle
guard is held.
