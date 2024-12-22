let wasm;

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_export_2.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_export_2.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_7.get(state.dtor)(state.a, state.b)
});

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_7.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_7.get(state.dtor)(state.a, state.b);
                state.a = 0;
                CLOSURE_DTORS.unregister(state);
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}
function __wbg_adapter_30(arg0, arg1, arg2) {
    wasm.closure554_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_33(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hc540aa29abaa6d22(arg0, arg1);
}

function __wbg_adapter_42(arg0, arg1, arg2) {
    wasm.closure828_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_45(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__he9883a2299e8813d(arg0, arg1);
}

function __wbg_adapter_56(arg0, arg1, arg2) {
    wasm.closure995_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_59(arg0, arg1, arg2) {
    wasm.closure1028_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_62(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h409d3a079c3679e5(arg0, arg1);
}

function __wbg_adapter_69(arg0, arg1, arg2) {
    wasm.closure1082_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_72(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h80637f3961062d7d(arg0, arg1);
}

function __wbg_adapter_75(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hcb92eb34671f016a(arg0, arg1);
}

function __wbg_adapter_78(arg0, arg1, arg2) {
    wasm.closure1146_externref_shim(arg0, arg1, arg2);
}

const __wbindgen_enum_BinaryType = ["blob", "arraybuffer"];

const __wbindgen_enum_RtcDataChannelState = ["connecting", "open", "closing", "closed"];

const __wbindgen_enum_RtcDataChannelType = ["arraybuffer", "blob"];

const __wbindgen_enum_RtcIceConnectionState = ["new", "checking", "connected", "completed", "failed", "disconnected", "closed"];

const __wbindgen_enum_RtcIceGatheringState = ["new", "gathering", "complete"];

const __wbindgen_enum_RtcPeerConnectionState = ["closed", "failed", "disconnected", "new", "connecting", "connected"];

const __wbindgen_enum_RtcSdpType = ["offer", "pranswer", "answer", "rollback"];

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_addEventListener_37872d53aeb4c65a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3, arg4);
    }, arguments) };
    imports.wbg.__wbg_addEventListener_b9481c2c2cab6047 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3);
    }, arguments) };
    imports.wbg.__wbg_addIceCandidate_dab4ba086e94a197 = function(arg0, arg1) {
        const ret = arg0.addIceCandidate(arg1);
        return ret;
    };
    imports.wbg.__wbg_addIceCandidate_e301d64de5bab94d = function(arg0, arg1) {
        const ret = arg0.addIceCandidate(arg1);
        return ret;
    };
    imports.wbg.__wbg_body_8d7d8c4aa91dcad8 = function(arg0) {
        const ret = arg0.body;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_bubbles_48182817f8ec169f = function(arg0) {
        const ret = arg0.bubbles;
        return ret;
    };
    imports.wbg.__wbg_buffer_61b7ce01341d7f88 = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_cachekey_57601dac16343711 = function(arg0) {
        const ret = arg0.__yew_subtree_cache_key;
        return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
    };
    imports.wbg.__wbg_call_500db948e69c7330 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.call(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_b0d8e36992d9900d = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_cancelBubble_4f3b0a3179912b26 = function(arg0) {
        const ret = arg0.cancelBubble;
        return ret;
    };
    imports.wbg.__wbg_candidate_3fa3d192f9eecd2e = function(arg0) {
        const ret = arg0.candidate;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_candidate_9e1d5a3a02912ef4 = function(arg0, arg1) {
        const ret = arg1.candidate;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_channel_fb425a8360df06ae = function(arg0) {
        const ret = arg0.channel;
        return ret;
    };
    imports.wbg.__wbg_childNodes_609b812b2d01cb84 = function(arg0) {
        const ret = arg0.childNodes;
        return ret;
    };
    imports.wbg.__wbg_clearInterval_dd1e598f425db353 = function(arg0) {
        const ret = clearInterval(arg0);
        return ret;
    };
    imports.wbg.__wbg_clearTimeout_5a54f8841c30079a = function(arg0) {
        const ret = clearTimeout(arg0);
        return ret;
    };
    imports.wbg.__wbg_clearTimeout_96804de0ab838f26 = function(arg0) {
        const ret = clearTimeout(arg0);
        return ret;
    };
    imports.wbg.__wbg_cloneNode_d247de63446fb130 = function() { return handleError(function (arg0) {
        const ret = arg0.cloneNode();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_close_4063e1bcbd6d5fe2 = function() { return handleError(function (arg0) {
        arg0.close();
    }, arguments) };
    imports.wbg.__wbg_close_80ce50bd282db748 = function(arg0) {
        arg0.close();
    };
    imports.wbg.__wbg_close_9a6122c0c6e33079 = function(arg0) {
        arg0.close();
    };
    imports.wbg.__wbg_code_71136b86e2aa7f41 = function(arg0) {
        const ret = arg0.code;
        return ret;
    };
    imports.wbg.__wbg_code_cd82312abb9d9ff9 = function(arg0) {
        const ret = arg0.code;
        return ret;
    };
    imports.wbg.__wbg_composedPath_3a3f292fe1666e00 = function(arg0) {
        const ret = arg0.composedPath();
        return ret;
    };
    imports.wbg.__wbg_connectionState_c2af96f7b9c346c1 = function(arg0) {
        const ret = arg0.connectionState;
        return (__wbindgen_enum_RtcPeerConnectionState.indexOf(ret) + 1 || 7) - 1;
    };
    imports.wbg.__wbg_createAnswer_9f830cf62287290e = function(arg0) {
        const ret = arg0.createAnswer();
        return ret;
    };
    imports.wbg.__wbg_createDataChannel_b0e7ba0ee7551b66 = function(arg0, arg1, arg2, arg3) {
        const ret = arg0.createDataChannel(getStringFromWasm0(arg1, arg2), arg3);
        return ret;
    };
    imports.wbg.__wbg_createDataChannel_b881394dc224365d = function(arg0, arg1, arg2) {
        const ret = arg0.createDataChannel(getStringFromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_createElementNS_494cc14f5fdee138 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = arg0.createElementNS(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_createElement_89923fcb809656b7 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_createOffer_77d10ea157cc1bae = function(arg0) {
        const ret = arg0.createOffer();
        return ret;
    };
    imports.wbg.__wbg_createTextNode_457c122eb9cb5753 = function(arg0, arg1, arg2) {
        const ret = arg0.createTextNode(getStringFromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_crypto_ed58b8e10a292839 = function(arg0) {
        const ret = arg0.crypto;
        return ret;
    };
    imports.wbg.__wbg_data_4ce8a82394d8b110 = function(arg0) {
        const ret = arg0.data;
        return ret;
    };
    imports.wbg.__wbg_debug_19114f11037e4658 = function(arg0, arg1, arg2, arg3) {
        console.debug(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_dispatchEvent_391667c727dc7c90 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.dispatchEvent(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_document_f11bc4f7c03e1745 = function(arg0) {
        const ret = arg0.document;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_error_3c7d958458bf649b = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4, 4);
        console.error(...v0);
    };
    imports.wbg.__wbg_error_483d659117b6f3f6 = function(arg0, arg1, arg2, arg3) {
        console.error(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_error_7534b8e9a36f1ab4 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_error_fab41a42d22bf2bc = function(arg0) {
        console.error(arg0);
    };
    imports.wbg.__wbg_from_d68eaa96dba25449 = function(arg0) {
        const ret = Array.from(arg0);
        return ret;
    };
    imports.wbg.__wbg_getRandomValues_bcb4912f16000dc4 = function() { return handleError(function (arg0, arg1) {
        arg0.getRandomValues(arg1);
    }, arguments) };
    imports.wbg.__wbg_get_9aa3dff3f0266054 = function(arg0, arg1) {
        const ret = arg0[arg1 >>> 0];
        return ret;
    };
    imports.wbg.__wbg_get_bbccf8970793c087 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_getsdp_62593a5c2574334e = function(arg0, arg1) {
        const ret = arg1.sdp;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_host_428e6a6189a92c91 = function(arg0) {
        const ret = arg0.host;
        return ret;
    };
    imports.wbg.__wbg_iceConnectionState_96d0e819375d3695 = function(arg0) {
        const ret = arg0.iceConnectionState;
        return (__wbindgen_enum_RtcIceConnectionState.indexOf(ret) + 1 || 8) - 1;
    };
    imports.wbg.__wbg_iceGatheringState_272d259248e39488 = function(arg0) {
        const ret = arg0.iceGatheringState;
        return (__wbindgen_enum_RtcIceGatheringState.indexOf(ret) + 1 || 4) - 1;
    };
    imports.wbg.__wbg_info_18e75e6ce8a36a90 = function(arg0, arg1, arg2, arg3) {
        console.info(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_insertBefore_4b32ede81cf15b43 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.insertBefore(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_instanceof_ArrayBuffer_670ddde44cdb2602 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Blob_2fb69097f32d6784 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Blob;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Element_0f1680908791f190 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Element;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Error_2b29c5b4afac4e22 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Error;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_ShadowRoot_184d5b63318b09e5 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ShadowRoot;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_d2514c6a7ee7ba60 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_is_e442492d1fb7967b = function(arg0, arg1) {
        const ret = Object.is(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbg_label_98357dccdd87d362 = function(arg0, arg1) {
        const ret = arg1.label;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_lastChild_a1995c15767ea782 = function(arg0) {
        const ret = arg0.lastChild;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_length_65d1cd11729ced11 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_d65cf0786bfc5739 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_listenerid_ed1678830a5b97ec = function(arg0) {
        const ret = arg0.__yew_listener_id;
        return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
    };
    imports.wbg.__wbg_localDescription_ac6bdbde1834d62e = function(arg0) {
        const ret = arg0.localDescription;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_log_bc77772961bf21bb = function(arg0, arg1, arg2, arg3) {
        console.log(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_message_7bde112094278773 = function(arg0) {
        const ret = arg0.message;
        return ret;
    };
    imports.wbg.__wbg_msCrypto_0a36e2ec3a343d26 = function(arg0) {
        const ret = arg0.msCrypto;
        return ret;
    };
    imports.wbg.__wbg_name_ae6b09babb81aa7d = function(arg0) {
        const ret = arg0.name;
        return ret;
    };
    imports.wbg.__wbg_namespaceURI_85b8a45764ae1d6e = function(arg0, arg1) {
        const ret = arg1.namespaceURI;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_new_254fa9eac11932ae = function() {
        const ret = new Array();
        return ret;
    };
    imports.wbg.__wbg_new_3ff5b33b1ce712df = function(arg0) {
        const ret = new Uint8Array(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_688846f374351c92 = function() {
        const ret = new Object();
        return ret;
    };
    imports.wbg.__wbg_new_8a6f238a6ece86ea = function() {
        const ret = new Error();
        return ret;
    };
    imports.wbg.__wbg_new_9b6c38191d7b9512 = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_aa395d3e4e157cde = function() { return handleError(function (arg0) {
        const ret = new RTCIceCandidate(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newnoargs_fd9e4bf8be2bc16d = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_ba35896968751d91 = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_newwithconfiguration_496479cef0f6908f = function() { return handleError(function (arg0) {
        const ret = new RTCPeerConnection(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwitheventinitdict_8ef7c889b37bf3b0 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new CloseEvent(getStringFromWasm0(arg0, arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwithlength_34ce8f1051e74449 = function(arg0) {
        const ret = new Uint8Array(arg0 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_newwithstrsequence_5b5601fc2c0bff30 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_nextSibling_f1b7916c306c9a9f = function(arg0) {
        const ret = arg0.nextSibling;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_node_02999533c4ea02e3 = function(arg0) {
        const ret = arg0.node;
        return ret;
    };
    imports.wbg.__wbg_now_64d0bb151e5d3889 = function() {
        const ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_outerHTML_23b92e7039b77dad = function(arg0, arg1) {
        const ret = arg1.outerHTML;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_parentElement_41c6f5f746ea74cf = function(arg0) {
        const ret = arg0.parentElement;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_parentNode_dfffcfb63c8a6cac = function(arg0) {
        const ret = arg0.parentNode;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_parse_161c68378e086ae1 = function() { return handleError(function (arg0, arg1) {
        const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_preventDefault_3c86e59772d015e6 = function(arg0) {
        arg0.preventDefault();
    };
    imports.wbg.__wbg_process_5c1d670bc53614b8 = function(arg0) {
        const ret = arg0.process;
        return ret;
    };
    imports.wbg.__wbg_push_6edad0df4b546b2c = function(arg0, arg1) {
        const ret = arg0.push(arg1);
        return ret;
    };
    imports.wbg.__wbg_queueMicrotask_2181040e064c0dc8 = function(arg0) {
        queueMicrotask(arg0);
    };
    imports.wbg.__wbg_queueMicrotask_ef9ac43769cbcc4f = function(arg0) {
        const ret = arg0.queueMicrotask;
        return ret;
    };
    imports.wbg.__wbg_randomFillSync_ab2cfe79ebbf2740 = function() { return handleError(function (arg0, arg1) {
        arg0.randomFillSync(arg1);
    }, arguments) };
    imports.wbg.__wbg_readyState_236b61903e1dbb47 = function(arg0) {
        const ret = arg0.readyState;
        return ret;
    };
    imports.wbg.__wbg_readyState_f177cc518c1b15b6 = function(arg0) {
        const ret = arg0.readyState;
        return (__wbindgen_enum_RtcDataChannelState.indexOf(ret) + 1 || 5) - 1;
    };
    imports.wbg.__wbg_reason_17a506fc63aa299a = function(arg0, arg1) {
        const ret = arg1.reason;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_removeAttribute_013475d9f18db70a = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.removeAttribute(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_removeChild_c6861558b785880c = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.removeChild(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_removeEventListener_a9ca9f05245321f0 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
    }, arguments) };
    imports.wbg.__wbg_require_79b1e9274cde3c87 = function() { return handleError(function () {
        const ret = module.require;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_resolve_0bf7c44d641804f9 = function(arg0) {
        const ret = Promise.resolve(arg0);
        return ret;
    };
    imports.wbg.__wbg_sdpMLineIndex_221943a59df8c42a = function(arg0) {
        const ret = arg0.sdpMLineIndex;
        return isLikeNone(ret) ? 0xFFFFFF : ret;
    };
    imports.wbg.__wbg_sdpMid_b3f632f30e652e71 = function(arg0, arg1) {
        const ret = arg1.sdpMid;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_sdp_b6dd8078e5da25d1 = function(arg0, arg1) {
        const ret = arg1.sdp;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_send_8d1796bdf62d7537 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_send_982ad087cc71fef9 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_send_c2b76ede40fcced1 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.send(getArrayU8FromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_send_c30f435bb2d77003 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.send(getArrayU8FromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_setAttribute_148e0e65e20e5f27 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setInterval_ed3b5e3c3ebb8a6d = function() { return handleError(function (arg0, arg1) {
        const ret = setInterval(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setLocalDescription_b788ec2fd0674a96 = function(arg0, arg1) {
        const ret = arg0.setLocalDescription(arg1);
        return ret;
    };
    imports.wbg.__wbg_setRemoteDescription_d669d99de7807ea2 = function(arg0, arg1) {
        const ret = arg0.setRemoteDescription(arg1);
        return ret;
    };
    imports.wbg.__wbg_setTimeout_db2dbaeefb6f39c7 = function() { return handleError(function (arg0, arg1) {
        const ret = setTimeout(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setTimeout_eefe7f4c234b0c6b = function() { return handleError(function (arg0, arg1) {
        const ret = setTimeout(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_1d80752d0d5f0b21 = function(arg0, arg1, arg2) {
        arg0[arg1 >>> 0] = arg2;
    };
    imports.wbg.__wbg_set_23d69db4e5c66a6e = function(arg0, arg1, arg2) {
        arg0.set(arg1, arg2 >>> 0);
    };
    imports.wbg.__wbg_set_3f1d0b984ed272ed = function(arg0, arg1, arg2) {
        arg0[arg1] = arg2;
    };
    imports.wbg.__wbg_set_4e647025551483bd = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(arg0, arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setbinaryType_2704a1179f1dae81 = function(arg0, arg1) {
        arg0.binaryType = __wbindgen_enum_RtcDataChannelType[arg1];
    };
    imports.wbg.__wbg_setbinaryType_3fa4a9e8d2cc506f = function(arg0, arg1) {
        arg0.binaryType = __wbindgen_enum_BinaryType[arg1];
    };
    imports.wbg.__wbg_setcachekey_bb5f908a0e3ee714 = function(arg0, arg1) {
        arg0.__yew_subtree_cache_key = arg1 >>> 0;
    };
    imports.wbg.__wbg_setcandidate_1bd42bca015a05a1 = function(arg0, arg1, arg2) {
        arg0.candidate = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setcapture_b1387485d0f7947c = function(arg0, arg1) {
        arg0.capture = arg1 !== 0;
    };
    imports.wbg.__wbg_setchecked_cc9be394960a6551 = function(arg0, arg1) {
        arg0.checked = arg1 !== 0;
    };
    imports.wbg.__wbg_setcode_6c2f9e460f442b5b = function(arg0, arg1) {
        arg0.code = arg1;
    };
    imports.wbg.__wbg_seticeservers_b00d802d70f18bf7 = function(arg0, arg1) {
        arg0.iceServers = arg1;
    };
    imports.wbg.__wbg_setid_1e31ae38dd199f1e = function(arg0, arg1) {
        arg0.id = arg1;
    };
    imports.wbg.__wbg_setinnerHTML_2d75307ba8832258 = function(arg0, arg1, arg2) {
        arg0.innerHTML = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setlistenerid_3d14d37a42484593 = function(arg0, arg1) {
        arg0.__yew_listener_id = arg1 >>> 0;
    };
    imports.wbg.__wbg_setmaxretransmits_6f61e654473f025b = function(arg0, arg1) {
        arg0.maxRetransmits = arg1;
    };
    imports.wbg.__wbg_setnegotiated_adeee8a35c80b239 = function(arg0, arg1) {
        arg0.negotiated = arg1 !== 0;
    };
    imports.wbg.__wbg_setnodeValue_2bdce51097cd14f6 = function(arg0, arg1, arg2) {
        arg0.nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setonce_87cf501e67ee47f7 = function(arg0, arg1) {
        arg0.once = arg1 !== 0;
    };
    imports.wbg.__wbg_setonclose_629a987007f094d6 = function(arg0, arg1) {
        arg0.onclose = arg1;
    };
    imports.wbg.__wbg_setonclose_f9c609d8c9938fa5 = function(arg0, arg1) {
        arg0.onclose = arg1;
    };
    imports.wbg.__wbg_setonconnectionstatechange_802cd72c5cdf8011 = function(arg0, arg1) {
        arg0.onconnectionstatechange = arg1;
    };
    imports.wbg.__wbg_setondatachannel_fb042d0774f63878 = function(arg0, arg1) {
        arg0.ondatachannel = arg1;
    };
    imports.wbg.__wbg_setonerror_63a911ab4d90e28b = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonerror_8ae2b387470ec52e = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonicecandidate_6efe8084e067fa0b = function(arg0, arg1) {
        arg0.onicecandidate = arg1;
    };
    imports.wbg.__wbg_setoniceconnectionstatechange_ccbe455983413d66 = function(arg0, arg1) {
        arg0.oniceconnectionstatechange = arg1;
    };
    imports.wbg.__wbg_setonicegatheringstatechange_6fb61f0de024221d = function(arg0, arg1) {
        arg0.onicegatheringstatechange = arg1;
    };
    imports.wbg.__wbg_setonmessage_5e7ade2af360de9d = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_setonmessage_b53b0e68a6823042 = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_setonnegotiationneeded_377f39d724d62130 = function(arg0, arg1) {
        arg0.onnegotiationneeded = arg1;
    };
    imports.wbg.__wbg_setonopen_54faa9e83483da1d = function(arg0, arg1) {
        arg0.onopen = arg1;
    };
    imports.wbg.__wbg_setonopen_be193e2f796876a6 = function(arg0, arg1) {
        arg0.onopen = arg1;
    };
    imports.wbg.__wbg_setordered_b25b9bae3e757f40 = function(arg0, arg1) {
        arg0.ordered = arg1 !== 0;
    };
    imports.wbg.__wbg_setpassive_4e263ba5bfca97c5 = function(arg0, arg1) {
        arg0.passive = arg1 !== 0;
    };
    imports.wbg.__wbg_setreason_6ba48376e1afb3d9 = function(arg0, arg1, arg2) {
        arg0.reason = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setsdp_d4eab241040f765d = function(arg0, arg1, arg2) {
        arg0.sdp = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setsdpmid_50d9d24a44132e38 = function(arg0, arg1, arg2) {
        arg0.sdpMid = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setsdpmlineindex_937125a3f19e7c74 = function(arg0, arg1) {
        arg0.sdpMLineIndex = arg1 === 0xFFFFFF ? undefined : arg1;
    };
    imports.wbg.__wbg_setsubtreeid_32b8ceff55862e29 = function(arg0, arg1) {
        arg0.__yew_subtree_id = arg1 >>> 0;
    };
    imports.wbg.__wbg_settype_935b601281898749 = function(arg0, arg1) {
        arg0.type = __wbindgen_enum_RtcSdpType[arg1];
    };
    imports.wbg.__wbg_setvalue_44c59c360ad57cf0 = function(arg0, arg1, arg2) {
        arg0.value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setvalue_88aa1f99f6500a1e = function(arg0, arg1, arg2) {
        arg0.value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_stack_0ed75d68575b0f3c = function(arg0, arg1) {
        const ret = arg1.stack;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_0be7472e492ad3e3 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_1a6eb482d12c9bfb = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_1dc398a895c82351 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_ae1c80c7eea8d64a = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_stringify_f4f701bc34ceda61 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_subarray_46adeb9b86949d12 = function(arg0, arg1, arg2) {
        const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_subtreeid_e65dfcc52d403fd9 = function(arg0) {
        const ret = arg0.__yew_subtree_id;
        return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
    };
    imports.wbg.__wbg_target_a8fe593e7ee79c21 = function(arg0) {
        const ret = arg0.target;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_textContent_593cb1d610df6a86 = function(arg0, arg1) {
        const ret = arg1.textContent;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_then_0438fad860fe38e1 = function(arg0, arg1) {
        const ret = arg0.then(arg1);
        return ret;
    };
    imports.wbg.__wbg_then_0ffafeddf0e182a4 = function(arg0, arg1, arg2) {
        const ret = arg0.then(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_toJSON_ffaaeab21d6c96ee = function(arg0) {
        const ret = arg0.toJSON();
        return ret;
    };
    imports.wbg.__wbg_toString_cbcf95f260c441ae = function(arg0) {
        const ret = arg0.toString();
        return ret;
    };
    imports.wbg.__wbg_url_e296f4f5d83d962b = function(arg0, arg1) {
        const ret = arg1.url;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_2adb5f0602e19ca9 = function(arg0, arg1) {
        const ret = arg1.value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_47fde8ea2d9fdcd5 = function(arg0, arg1) {
        const ret = arg1.value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_a8b8b65bc31190d6 = function(arg0, arg1) {
        const ret = arg1.value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_versions_c71aa1626a93e0a1 = function(arg0) {
        const ret = arg0.versions;
        return ret;
    };
    imports.wbg.__wbg_warn_cb8be8bbf790a5d6 = function(arg0, arg1, arg2, arg3) {
        console.warn(arg0, arg1, arg2, arg3);
    };
    imports.wbg.__wbg_wasClean_303f938d5142dcca = function(arg0) {
        const ret = arg0.wasClean;
        return ret;
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = arg0.original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1504 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 829, __wbg_adapter_42);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1506 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 829, __wbg_adapter_45);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1508 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 829, __wbg_adapter_42);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1510 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 829, __wbg_adapter_42);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1512 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 829, __wbg_adapter_42);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1514 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 829, __wbg_adapter_42);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1845 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 996, __wbg_adapter_56);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1914 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1029, __wbg_adapter_59);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1916 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1029, __wbg_adapter_62);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1918 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1029, __wbg_adapter_59);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1920 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1029, __wbg_adapter_59);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2163 = function(arg0, arg1, arg2) {
        const ret = makeClosure(arg0, arg1, 1083, __wbg_adapter_69);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2276 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1131, __wbg_adapter_72);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2302 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1140, __wbg_adapter_75);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2318 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 1147, __wbg_adapter_78);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper925 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 555, __wbg_adapter_30);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper927 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 555, __wbg_adapter_33);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper929 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 555, __wbg_adapter_30);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper931 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 555, __wbg_adapter_30);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper933 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 555, __wbg_adapter_30);
        return ret;
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_export_2;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(arg0) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = arg0 === null;
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = arg0;
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(arg0) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return ret;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;


    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('konnekt_session_app_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
