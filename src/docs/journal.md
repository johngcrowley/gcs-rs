Currently, I have to do dynamic dispatch because TokenProvider is a trait, not a concrete type.

But, I'm using
[CustomServiceAccount](https://docs.rs/gcp_auth/latest/src/gcp_auth/custom_service_account.rs.html#130)
concrete type (from `gcp_auth` crate), so i don't really need dynamic dispatch.

I also don't have to worry about code bloat with monomorphization because I'm only ever going to
use 'CustomServiceAccount' concrete type, so I can choose to use Generics here as my
solution.

Instead of passing around an Arc< dyn ...>, I can make a struct like

```rust
struct HttpClient<T: TokenProvider> {
    token_provider: Arc<T>
}

impl<T: TokenProvider> for HttpClient<T> {
    fn upload {}
    fn download {}
}
```

etc, where that will compile down to just 'CustomServiceAccount'. Now, I'm not getting a
run-time cost, and I'm getting to pass around ownership, and I'm getting to call my
`.token()` method for refresh logic.


`TokenProvider` is a trait.

They impl it for `CustomServiceAccount` here:
- https://docs.rs/gcp_auth/latest/src/gcp_auth/custom_service_account.rs.html#130-157

so when im calling `.token()` it's really checking if it has expired yet.

So i want to pass around my provider which stores my state of my token (cached).


-- Get URI for resumable uploads --

Interesting error I had gotten when method-chaining `to_str()` to be a one-liner:
---------------------------------------------------------------------------------------------
                   "temporary value dropped while borrowed"
---------------------------------------------------------------------------------------------
The owned value of type `HeaderValue` which returns from `get_resumable_upload_uri()`
never made a pointer back to this stack frame (function). It's arm was groping from the
pit, but fell, limply. The `to_str()` turned it's owned value return into a reference,
which was dangling back to the stack frame it just exited. Thus, I must `let uri` BE
`uri`. Then, from this stack frame, I may play with it.



