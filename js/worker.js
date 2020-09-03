'use strict';

/* This section belongs in your existing source files
let cached_wasm;

function spawn_worker(id, stack_top) {
    worker = new Webworker('js/worker.js');

    worker.postMessage([cached_wasm, id, stack_top]);

}*/

onmessage = async function({ data }) {
    data = data[0]
    id = data[1]
    stack_top = data[2]
    wasm = await WebAssembly.instantiate(data.compiled, {env: {
        memory: mem
    }});

    wasm.exports.__sp.value = stack_top;
    wasm.exports.__wasm_init_memory();
    wasm.exports.__wasm_init_tls();
    wasm.exports.init(wasm.exports.__heap_base.value);
    wasm.exports.child_entry_point();
}

