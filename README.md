# ArenaCity

ArenaCity is an arena container which create Citizen that once drop, returns automatically to the Arena.
This is useful to reduce allocation of same type of items.
The Arena is safe for multi-thread context and a lock is applied only when seeking for an existing
Citizen or returning a Citizen to the arena.

```rust
use arena_city::ArenaCity;

let city = ArenaCity::new();
let mut foo = city.get_or_create(|| Vec::new());

// do some work with the vec.
foo.push(10);

// when the vec is out of scope, it will be sanitized and returned to the ArenaCity.
drop(foo);


// returns the dropped vec from the arena, does not create it.
let foo = city.get_or_create(|| unreachable!("it will reuse foo"));

assert_eq!(foo.len(), 0); 
```
