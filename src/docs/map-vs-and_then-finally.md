
`map`'s _closure_ returns just the type, but map itself will wrap.
so if you chained maps on just `x*2`
-  The closure is explicitly responsible for handling the potential for failure and
returning the appropriate wrapper.
- The distinction isn't about whether failure is possible, but rather about where the possibility of failure is introduced.
Map:
- the closure is not responsible for returning success or failure.
- it is expected to be an _infallbile_ transformation.
- hence the `FnOnce where: Option<T> -> U` bound.
- at the end of any transformation, you get back what you get back from the Closure, and
it's wrapped in `map`'s output type.
- if you return a `Some()` or a `Ok()` from your closure, it's getting nested in `map`'s.

And Then:
- the closure bears the responsibility of handling failure.
- these transformations _are fallible_.
- each `and_then` returns a new Ok() or Some(), which the next one operates on.

Aha:
- and_then: the return type of the closure _is_ the return type of the function, and_then.
- map: the return type of the  closure is U and the function is Option<U>, so if you
make 'U' (in the closure) be `Some(U)`, then that gets `Some(Some(U))`

```
fn main() {

    let x = Some(47);
    
    // closure returns a new 'Some()' + map returns 'Some()'
    let y = x.map(|i| Some(i));
    println!("{:?}", y);
    // Some(Some(47))
    
    // closure returns a new 'Some()'
    let z = x.and_then(|i| Some(i));
    println!("{:?}", z);
    // Some(47)

}
```
