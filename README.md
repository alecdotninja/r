# r

`r` is an experimental Rust crate for fractional ownership of a `Box`
allocation.

The experiment is about how shared ownership might behave if the ownership
count moved further into the type system:

```text
Arc<T> -> Rc<T> -> R<T, O>
```

`Arc<T>` supports shared ownership across threads with atomic runtime reference
counts. `Rc<T>` keeps shared ownership single-threaded and uses non-atomic
runtime reference counts. `R<T, O>` explores the next step: encode the ownership
share in the type, split and join it explicitly, and avoid a runtime reference
count.

The crate exposes `R<T, O>`, a pointer type whose const generic `O` tracks how
much of the allocation is owned by that handle. A value starts as `R<T, FULL>`.
Full ownership is unique, can be converted back into a `Box<T>`, and drops the
allocation when it is dropped. A handle can be split into two equal shares, which
can later be joined back together.

```rust
use r::R;

let value = R::new(7);
let (left, right) = R::split(value);

assert_eq!(*left, 7);
assert_eq!(*right, 7);

let value = R::join(left, right);
assert_eq!(R::into_inner(value), 7);
```

Shared ownership handles dereference to `&T`. Only `R<T, FULL>` can dereference
mutably or consume the allocation, because only full ownership represents a
unique handle. Partial ownership handles must be joined back together or leaked
intentionally; dropping a partial handle is a logic error and panics.

`R::join` panics if the handles point to different allocations. Use
`R::try_join` when a mismatch is possible and the handles need to be recovered.

## Why not just shared references?

Unlike regular shared references, split `R` handles can be moved into unscoped
threads and later rejoined into full ownership. That makes it possible to share
a value, do work in separate owners, and then recover the owned value afterward.

```rust
use r::R;
use std::sync::Mutex;

let value = R::new(Mutex::new(0));
let (left, right) = R::split(value);

let left = std::thread::spawn(move || {
    *left.lock().unwrap() += 1;
    left
})
.join()
.unwrap();

let right = std::thread::spawn(move || {
    *right.lock().unwrap() += 1;
    right
})
.join()
.unwrap();

let value = R::join(left, right);
let mutex = R::into_inner(value);

assert_eq!(mutex.into_inner().unwrap(), 2);
```

The analogous plain-reference version cannot be written with
`std::thread::spawn`, because the spawned closures must be `'static`:

```rust,compile_fail
use std::sync::Mutex;

let mutex = Mutex::new(0);
let left = &mutex;
let right = &mutex;

let left = std::thread::spawn(move || {
    *left.lock().unwrap() += 1;
    left
})
.join()
.unwrap();

let right = std::thread::spawn(move || {
    *right.lock().unwrap() += 1;
    right
})
.join()
.unwrap();

assert!(std::ptr::eq(left, right));
assert_eq!(mutex.into_inner().unwrap(), 2);
```

Scoped threads can express a borrow-based version of this pattern. The
difference is that scoped references remain tied to the stack frame that owns
the value, while `R` moves the ownership proof itself and rebuilds full
ownership by joining the pieces.

## Requirements

This crate currently requires nightly Rust because it uses
`generic_const_exprs`.

## Safety

`R::from_raw` is unsafe. The pointer must come from `Box::into_raw`, and the
caller must provide the correct ownership marker for the handle being
constructed. Prefer `R::new`, `R::from_box`, `R::split`, and `R::join` when
possible.
