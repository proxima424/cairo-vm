# Demo of `cairo-rs` on WebAssembly

While cairo-rs is compatible with WebAssembly, it doesn't implement any bindings to it.
Instead, create a new WebAssembly crate with cairo-rs as a dependency and implement the required functionality there.

Since mimalloc is not automatically compilable to WebAssembly, the cairo-rs dependency should disable the default features, which will in turn disable mimalloc.

A working example is provided in this repository.

## Building

To build the example, run:

```sh
wasm-pack build --target=web
```

This will generate a javascript module that is directly loadable by the browser.

## Running

To run the example webpage, you need to run an HTTP server.
For example, using the _live-server_ npm module:

```sh
# while in <repo>/examples/wasm-demo
npx live-server
```

> **Warning**
> Trying to run `index.html` directly (i.e. URL starts with `file://`) will result in a CORS error.
