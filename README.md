# Web Worker

Rayon-like worker thread pool. Heavily inspired and based on the works of [Alex Crichton](https://github.com/alexcrichton)
This Fork will remove the wasm-bindgen debendency and try to improve performance by using `atomic.wait()` and `atomic.notify()`

[Original example](https://github.com/rustwasm/wasm-bindgen/tree/master/examples/raytrace-parallel)
[Generalized Version](https://github.com/amethyst/web_worker)
