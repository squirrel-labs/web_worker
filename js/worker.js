'use strict';

/* This section belongs in your existing source files
let cached_wasm;
let mem;

function spawn_worker(id, stack_top) {
    worker = new Worker('js/worker.js');

    worker.postMessage([cached_wasm, mem, id, stack_top]);

}*/

onmessage = async function({ data }) {
    data = data[0]
    mem = data[1]
    id = data[2]
    stack_top = data[3]
    wasm = await WebAssembly.instantiate(data.compiled, {env: {
        memory: mem
    }});

    wasm.exports.__sp.value = stack_top;
    wasm.exports.__wasm_init_memory();
    wasm.exports.__wasm_init_tls();
    wasm.exports.init(wasm.exports.__heap_base.value);
    wasm.exports.child_entry_point();
}

