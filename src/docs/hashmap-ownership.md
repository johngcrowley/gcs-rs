// even though function gets this passed by value, function parameters in Rust are
// immutable by default.
metadata
    .as_mut()
    .map(|m| m.0.entry("name".to_string()).or_insert(to.to_string()));

// this works because as_mut is a mutable reference to metadata, not the Option around it.
// so the closure gets passed a reference -- it doesn't own anything.
metadata
    .clone()
    .as_mut()
    .map(|m| m.0.entry("name".to_string()).or_insert(to.to_string()));

// Error: "cannot return value referencing local data `m.0`  returns a value referencing data owned by the current function"
// this moves metadata into the closure, unwrapping the option.
// or_insert() returns a reference to m.0 which is dropped, along with m, after the closure
// exits. even if we "let stuff = " its still a dangling reference.
metadata.map(|mut m| m.0.entry("name".to_string()).or_insert(to.to_string()));

