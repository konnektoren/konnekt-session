let $=`same-origin`,S=`function`,a3=`console.error`,Y=`Object`,Q=0,a0=`default`,W=`string`,U=1,a2=`error`,a5=623,V=3,P=null,a1=`cors`,N=`utf-8`,M=`undefined`,_=4,X=Array,O=Error,Z=FinalizationRegistry,a4=Object,a6=Object.getPrototypeOf,R=Uint8Array,T=undefined;var K=(b=>{if(a!==T)return a;if(typeof b!==M){if(a6(b)===a4.prototype){({module:b}=b)}else{console.warn(`using deprecated parameters for \`initSync()\`; pass a single object instead`)}};const c=H();I(c);if(!(b instanceof WebAssembly.Module)){b=new WebAssembly.Module(b)};const d=new WebAssembly.Instance(b,c);return J(d,b)});var H=(()=>{const b={};b.wbg={};b.wbg.__wbindgen_string_new=((a,b)=>{const c=e(a,b);return c});b.wbg.__wbindgen_cb_drop=(a=>{const b=a.original;if(b.cnt--==U){b.a=Q;return !0};const c=!1;return c});b.wbg.__wbindgen_is_string=(a=>{const b=typeof a===W;return b});b.wbg.__wbindgen_string_get=((b,c)=>{const d=c;const e=typeof d===W?d:T;var g=j(e)?Q:i(e,a.__wbindgen_malloc,a.__wbindgen_realloc);var h=f;l().setInt32(b+ _*U,h,!0);l().setInt32(b+ _*Q,g,!0)});b.wbg.__wbg_cachekey_b81c1aacc6a0645c=((a,b)=>{const c=b.__yew_subtree_cache_key;l().setInt32(a+ _*U,j(c)?Q:c,!0);l().setInt32(a+ _*Q,!j(c),!0)});b.wbg.__wbg_subtreeid_e80a1798fee782f9=((a,b)=>{const c=b.__yew_subtree_id;l().setInt32(a+ _*U,j(c)?Q:c,!0);l().setInt32(a+ _*Q,!j(c),!0)});b.wbg.__wbg_setsubtreeid_e1fab6b578c800cf=((a,b)=>{a.__yew_subtree_id=b>>>Q});b.wbg.__wbg_setcachekey_75bcd45312087529=((a,b)=>{a.__yew_subtree_cache_key=b>>>Q});b.wbg.__wbg_setlistenerid_f2e783343fa0cec1=((a,b)=>{a.__yew_listener_id=b>>>Q});b.wbg.__wbg_listenerid_6dcf1c62b7b7de58=((a,b)=>{const c=b.__yew_listener_id;l().setInt32(a+ _*U,j(c)?Q:c,!0);l().setInt32(a+ _*Q,!j(c),!0)});b.wbg.__wbg_new_abda76e883ba8a5f=(()=>{const a=new O();return a});b.wbg.__wbg_stack_658279fe44541cf6=((b,c)=>{const d=c.stack;const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_error_f851667af71bcfc6=((b,c)=>{let d;let f;try{d=b;f=c;console.error(e(b,c))}finally{a.__wbindgen_free(d,f,U)}});b.wbg.__wbg_queueMicrotask_848aa4969108a57e=(a=>{const b=a.queueMicrotask;return b});b.wbg.__wbindgen_is_function=(a=>{const b=typeof a===S;return b});b.wbg.__wbg_queueMicrotask_c5419c06eab41e73=typeof queueMicrotask==S?queueMicrotask:u(`queueMicrotask`);b.wbg.__wbg_crypto_1d1f22824a6a080c=(a=>{const b=a.crypto;return b});b.wbg.__wbindgen_is_object=(a=>{const b=a;const c=typeof b===`object`&&b!==P;return c});b.wbg.__wbg_process_4a72847cc503995b=(a=>{const b=a.process;return b});b.wbg.__wbg_versions_f686565e586dd935=(a=>{const b=a.versions;return b});b.wbg.__wbg_node_104a2ff8d6ea03a2=(a=>{const b=a.node;return b});b.wbg.__wbg_require_cca90b1a94a0255b=function(){return w((()=>{const a=module.require;return a}),arguments)};b.wbg.__wbg_msCrypto_eb05e62b530a1508=(a=>{const b=a.msCrypto;return b});b.wbg.__wbg_randomFillSync_5c9c955aa56b6049=function(){return w(((a,b)=>{a.randomFillSync(b)}),arguments)};b.wbg.__wbg_getRandomValues_3aa56aa6edec874c=function(){return w(((a,b)=>{a.getRandomValues(b)}),arguments)};b.wbg.__wbg_error_a526fb08a0205972=((b,c)=>{var d=x(b,c).slice();a.__wbindgen_free(b,c*_,_);console.error(...d)});b.wbg.__wbg_instanceof_Window_6575cd7f1322f82f=(a=>{let b;try{b=a instanceof Window}catch(a){b=!1}const c=b;return c});b.wbg.__wbg_document_d7fa2c739c2b191a=(a=>{const b=a.document;return j(b)?Q:v(b)});b.wbg.__wbg_body_8e909b791b1745d3=(a=>{const b=a.body;return j(b)?Q:v(b)});b.wbg.__wbg_createElement_e4523490bd0ae51d=function(){return w(((a,b,c)=>{const d=a.createElement(e(b,c));return d}),arguments)};b.wbg.__wbg_createElementNS_e51a368ab3a64b37=function(){return w(((a,b,c,d,f)=>{const g=a.createElementNS(b===Q?T:e(b,c),e(d,f));return g}),arguments)};b.wbg.__wbg_createTextNode_3b33c97f8ef3e999=((a,b,c)=>{const d=a.createTextNode(e(b,c));return d});b.wbg.__wbg_instanceof_Element_1a81366cc90e70e2=(a=>{let b;try{b=a instanceof Element}catch(a){b=!1}const c=b;return c});b.wbg.__wbg_namespaceURI_dc264d717ce10acb=((b,c)=>{const d=c.namespaceURI;var e=j(d)?Q:i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);var g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_setinnerHTML_559d45055154f1d8=((a,b,c)=>{a.innerHTML=e(b,c)});b.wbg.__wbg_outerHTML_02fdcad893a01b32=((b,c)=>{const d=c.outerHTML;const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_removeAttribute_2dc68056b5e6ea3d=function(){return w(((a,b,c)=>{a.removeAttribute(e(b,c))}),arguments)};b.wbg.__wbg_setAttribute_2a8f647a8d92c712=function(){return w(((a,b,c,d,f)=>{a.setAttribute(e(b,c),e(d,f))}),arguments)};b.wbg.__wbg_setcapture_216080a2de0d127c=((a,b)=>{a.capture=b!==Q});b.wbg.__wbg_setonce_9f2ce9d61cf01425=((a,b)=>{a.once=b!==Q});b.wbg.__wbg_setpassive_b1f337166a79f6c5=((a,b)=>{a.passive=b!==Q});b.wbg.__wbg_target_b0499015ea29563d=(a=>{const b=a.target;return j(b)?Q:v(b)});b.wbg.__wbg_bubbles_c48a1056384e852c=(a=>{const b=a.bubbles;return b});b.wbg.__wbg_cancelBubble_1fc3632e2ba513ed=(a=>{const b=a.cancelBubble;return b});b.wbg.__wbg_composedPath_d27a772830ab5dd0=(a=>{const b=a.composedPath();return b});b.wbg.__wbg_preventDefault_eecc4a63e64c4526=(a=>{a.preventDefault()});b.wbg.__wbg_value_a8d0480de0da39cf=((b,c)=>{const d=c.value;const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_setvalue_b68cd0e5fd3eb17f=((a,b,c)=>{a.value=e(b,c)});b.wbg.__wbg_addEventListener_4357f9b7b3826784=function(){return w(((a,b,c,d)=>{a.addEventListener(e(b,c),d)}),arguments)};b.wbg.__wbg_addEventListener_0ac72681badaf1aa=function(){return w(((a,b,c,d,f)=>{a.addEventListener(e(b,c),d,f)}),arguments)};b.wbg.__wbg_dispatchEvent_d3978479884f576d=function(){return w(((a,b)=>{const c=a.dispatchEvent(b);return c}),arguments)};b.wbg.__wbg_removeEventListener_4c13d11156153514=function(){return w(((a,b,c,d)=>{a.removeEventListener(e(b,c),d)}),arguments)};b.wbg.__wbg_setchecked_0b332e38c9022183=((a,b)=>{a.checked=b!==Q});b.wbg.__wbg_value_0cffd86fb9a5a18d=((b,c)=>{const d=c.value;const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_setvalue_36bcf6f86c998f0a=((a,b,c)=>{a.value=e(b,c)});b.wbg.__wbg_wasClean_cf2135191288f963=(a=>{const b=a.wasClean;return b});b.wbg.__wbg_code_9d4413f8b44b70c2=(a=>{const b=a.code;return b});b.wbg.__wbg_reason_ae1d72dfda13e899=((b,c)=>{const d=c.reason;const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_newwitheventinitdict_e04d4cf36ab15962=function(){return w(((a,b,c)=>{const d=new CloseEvent(e(a,b),c);return d}),arguments)};b.wbg.__wbg_parentNode_7e7d8adc9b41ce58=(a=>{const b=a.parentNode;return j(b)?Q:v(b)});b.wbg.__wbg_parentElement_bf013e6093029477=(a=>{const b=a.parentElement;return j(b)?Q:v(b)});b.wbg.__wbg_childNodes_87c5e311593a6192=(a=>{const b=a.childNodes;return b});b.wbg.__wbg_lastChild_d6a3eebc8b3cdd8c=(a=>{const b=a.lastChild;return j(b)?Q:v(b)});b.wbg.__wbg_nextSibling_46da01c3a2ce3774=(a=>{const b=a.nextSibling;return j(b)?Q:v(b)});b.wbg.__wbg_setnodeValue_ddb802810d61c610=((a,b,c)=>{a.nodeValue=b===Q?T:e(b,c)});b.wbg.__wbg_textContent_389dd460500a44bd=((b,c)=>{const d=c.textContent;var e=j(d)?Q:i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);var g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_cloneNode_bd4b7e47afe3ce9f=function(){return w((a=>{const b=a.cloneNode();return b}),arguments)};b.wbg.__wbg_insertBefore_5caa6ab4d3d6b481=function(){return w(((a,b,c)=>{const d=a.insertBefore(b,c);return d}),arguments)};b.wbg.__wbg_removeChild_aa85e67649730769=function(){return w(((a,b)=>{const c=a.removeChild(b);return c}),arguments)};b.wbg.__wbg_instanceof_ShadowRoot_6d00cedbc919c9a6=(a=>{let b;try{b=a instanceof ShadowRoot}catch(a){b=!1}const c=b;return c});b.wbg.__wbg_host_4a0b95cc36a45cb6=(a=>{const b=a.host;return b});b.wbg.__wbg_data_134d3a704b9fca32=(a=>{const b=a.data;return b});b.wbg.__wbg_value_0b0cebe9335a78ae=((b,c)=>{const d=c.value;const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbg_debug_a0b6c2c5ac9a4bfd=typeof console.debug==S?console.debug:u(`console.debug`);b.wbg.__wbg_error_53abcd6a461f73d8=typeof console.error==S?console.error:u(a3);b.wbg.__wbg_error_4d17c5bb1ca90c94=typeof console.error==S?console.error:u(a3);b.wbg.__wbg_info_1c7fba7da21072d1=typeof console.info==S?console.info:u(`console.info`);b.wbg.__wbg_log_4de37a0274d94769=typeof console.log==S?console.log:u(`console.log`);b.wbg.__wbg_warn_2e2787d40aad9a81=typeof console.warn==S?console.warn:u(`console.warn`);b.wbg.__wbg_setcode_a0c5900000499842=((a,b)=>{a.code=b});b.wbg.__wbg_setreason_7efb82dfa8a2f404=((a,b,c)=>{a.reason=e(b,c)});b.wbg.__wbg_readyState_bc0231e8c43b0907=(a=>{const b=a.readyState;return b});b.wbg.__wbg_setbinaryType_2befea8ba88b61e2=((a,b)=>{a.binaryType=z[b]});b.wbg.__wbg_new_d550f7a7120dd942=function(){return w(((a,b)=>{const c=new WebSocket(e(a,b));return c}),arguments)};b.wbg.__wbg_close_9e3b743c528a8d31=function(){return w((a=>{a.close()}),arguments)};b.wbg.__wbg_send_f308b110e144e90d=function(){return w(((a,b,c)=>{a.send(e(b,c))}),arguments)};b.wbg.__wbg_send_fe006eb24f5e2694=function(){return w(((a,b,c)=>{a.send(y(b,c))}),arguments)};b.wbg.__wbg_get_5419cf6b954aa11d=((a,b)=>{const c=a[b>>>Q];return c});b.wbg.__wbg_length_f217bbbf7e8e4df4=(a=>{const b=a.length;return b});b.wbg.__wbg_newnoargs_1ede4bf2ebbaaf43=((a,b)=>{const c=new Function(e(a,b));return c});b.wbg.__wbg_call_a9ef466721e824f2=function(){return w(((a,b)=>{const c=a.call(b);return c}),arguments)};b.wbg.__wbg_new_e69b5f66fda8f13c=(()=>{const a=new a4();return a});b.wbg.__wbg_self_bf91bf94d9e04084=function(){return w((()=>{const a=self.self;return a}),arguments)};b.wbg.__wbg_window_52dd9f07d03fd5f8=function(){return w((()=>{const a=window.window;return a}),arguments)};b.wbg.__wbg_globalThis_05c129bf37fcf1be=function(){return w((()=>{const a=globalThis.globalThis;return a}),arguments)};b.wbg.__wbg_global_3eca19bb09e9c484=function(){return w((()=>{const a=global.global;return a}),arguments)};b.wbg.__wbindgen_is_undefined=(a=>{const b=a===T;return b});b.wbg.__wbg_from_91a67a5f04c98a54=(a=>{const b=X.from(a);return b});b.wbg.__wbg_instanceof_ArrayBuffer_74945570b4a62ec7=(a=>{let b;try{b=a instanceof ArrayBuffer}catch(a){b=!1}const c=b;return c});b.wbg.__wbg_instanceof_Error_a0af335a62107964=(a=>{let b;try{b=a instanceof O}catch(a){b=!1}const c=b;return c});b.wbg.__wbg_message_00eebca8fa4dd7db=(a=>{const b=a.message;return b});b.wbg.__wbg_name_aa32a0ae51232604=(a=>{const b=a.name;return b});b.wbg.__wbg_toString_4b677455b9167e31=(a=>{const b=a.toString();return b});b.wbg.__wbg_call_3bfa248576352471=function(){return w(((a,b,c)=>{const d=a.call(b,c);return d}),arguments)};b.wbg.__wbg_is_4b64bc96710d6a0f=((a,b)=>{const c=a4.is(a,b);return c});b.wbg.__wbg_resolve_0aad7c1484731c99=(a=>{const b=Promise.resolve(a);return b});b.wbg.__wbg_then_748f75edfb032440=((a,b)=>{const c=a.then(b);return c});b.wbg.__wbg_buffer_ccaed51a635d8a2d=(a=>{const b=a.buffer;return b});b.wbg.__wbg_newwithbyteoffsetandlength_7e3eb787208af730=((a,b,c)=>{const d=new R(a,b>>>Q,c>>>Q);return d});b.wbg.__wbg_new_fec2611eb9180f95=(a=>{const b=new R(a);return b});b.wbg.__wbg_set_ec2fcf81bc573fd9=((a,b,c)=>{a.set(b,c>>>Q)});b.wbg.__wbg_length_9254c4bd3b9f23c4=(a=>{const b=a.length;return b});b.wbg.__wbg_newwithlength_76462a666eca145f=(a=>{const b=new R(a>>>Q);return b});b.wbg.__wbg_subarray_975a06f9dbd16995=((a,b,c)=>{const d=a.subarray(b>>>Q,c>>>Q);return d});b.wbg.__wbg_set_e864d25d9b399c9f=function(){return w(((a,b,c)=>{const d=Reflect.set(a,b,c);return d}),arguments)};b.wbg.__wbindgen_debug_string=((b,c)=>{const d=m(c);const e=i(d,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=f;l().setInt32(b+ _*U,g,!0);l().setInt32(b+ _*Q,e,!0)});b.wbg.__wbindgen_throw=((a,b)=>{throw new O(e(a,b))});b.wbg.__wbindgen_memory=(()=>{const b=a.memory;return b});b.wbg.__wbindgen_closure_wrapper1030=((a,b,c)=>{const d=o(a,b,a5,p);return d});b.wbg.__wbindgen_closure_wrapper1032=((a,b,c)=>{const d=o(a,b,a5,p);return d});b.wbg.__wbindgen_closure_wrapper1034=((a,b,c)=>{const d=o(a,b,a5,p);return d});b.wbg.__wbindgen_closure_wrapper1036=((a,b,c)=>{const d=o(a,b,a5,q);return d});b.wbg.__wbindgen_closure_wrapper1070=((a,b,c)=>{const d=r(a,b,632,s);return d});b.wbg.__wbindgen_closure_wrapper1360=((a,b,c)=>{const d=o(a,b,719,t);return d});b.wbg.__wbindgen_init_externref_table=(()=>{const b=a.__wbindgen_export_2;const c=b.grow(_);b.set(Q,T);b.set(c+ Q,T);b.set(c+ U,P);b.set(c+ 2,!0);b.set(c+ V,!1)});return b});var r=((b,c,d,e)=>{const f={a:b,b:c,cnt:U,dtor:d};const g=(...b)=>{f.cnt++;try{return e(f.a,f.b,...b)}finally{if(--f.cnt===Q){a.__wbindgen_export_3.get(f.dtor)(f.a,f.b);f.a=Q;n.unregister(f)}}};g.original=f;n.register(g,f,f);return g});var v=(b=>{const c=a.__externref_table_alloc();a.__wbindgen_export_2.set(c,b);return c});var d=(()=>{if(c===P||c.byteLength===Q){c=new R(a.memory.buffer)};return c});var s=((b,c,d)=>{a.closure631_externref_shim(b,c,d)});var e=((a,c)=>{a=a>>>Q;return b.decode(d().subarray(a,a+ c))});var j=(a=>a===T||a===P);var o=((b,c,d,e)=>{const f={a:b,b:c,cnt:U,dtor:d};const g=(...b)=>{f.cnt++;const c=f.a;f.a=Q;try{return e(c,f.b,...b)}finally{if(--f.cnt===Q){a.__wbindgen_export_3.get(f.dtor)(c,f.b);n.unregister(f)}else{f.a=c}}};g.original=f;n.register(g,f,f);return g});var t=((b,c,d)=>{a.closure718_externref_shim(b,c,d)});var l=(()=>{if(k===P||k.buffer.detached===!0||k.buffer.detached===T&&k.buffer!==a.memory.buffer){k=new DataView(a.memory.buffer)};return k});var u=(a=>()=>{throw new O(`${a} is not defined`)});var p=((b,c,d)=>{a.closure622_externref_shim(b,c,d)});var L=(async(b)=>{if(a!==T)return a;if(typeof b!==M){if(a6(b)===a4.prototype){({module_or_path:b}=b)}else{console.warn(`using deprecated parameters for the initialization function; pass a single object instead`)}};if(typeof b===M){b=new URL(`konnekt_session_app_bg.wasm`,import.meta.url)};const c=H();if(typeof b===W||typeof Request===S&&b instanceof Request||typeof URL===S&&b instanceof URL){b=fetch(b)};I(c);const {instance:d,module:e}=await G(await b,c);return J(d,e)});var m=(a=>{const b=typeof a;if(b==`number`||b==`boolean`||a==P){return `${a}`};if(b==W){return `"${a}"`};if(b==`symbol`){const b=a.description;if(b==P){return `Symbol`}else{return `Symbol(${b})`}};if(b==S){const b=a.name;if(typeof b==W&&b.length>Q){return `Function(${b})`}else{return `Function`}};if(X.isArray(a)){const b=a.length;let c=`[`;if(b>Q){c+=m(a[Q])};for(let d=U;d<b;d++){c+=`, `+ m(a[d])};c+=`]`;return c};const c=/\[object ([^\]]+)\]/.exec(toString.call(a));let d;if(c.length>U){d=c[U]}else{return toString.call(a)};if(d==Y){try{return `Object(`+ JSON.stringify(a)+ `)`}catch(a){return Y}};if(a instanceof O){return `${a.name}: ${a.message}\n${a.stack}`};return d});var x=((b,c)=>{b=b>>>Q;const d=l();const e=[];for(let f=b;f<b+ _*c;f+=_){e.push(a.__wbindgen_export_2.get(d.getUint32(f,!0)))};a.__externref_drop_slice(b,c);return e});var y=((a,b)=>{a=a>>>Q;return d().subarray(a/U,a/U+ b)});var G=(async(a,b)=>{if(typeof Response===S&&a instanceof Response){if(typeof WebAssembly.instantiateStreaming===S){try{return await WebAssembly.instantiateStreaming(a,b)}catch(b){if(a.headers.get(`Content-Type`)!=`application/wasm`){console.warn(`\`WebAssembly.instantiateStreaming\` failed because your server does not serve Wasm with \`application/wasm\` MIME type. Falling back to \`WebAssembly.instantiate\` which is slower. Original error:\\n`,b)}else{throw b}}};const c=await a.arrayBuffer();return await WebAssembly.instantiate(c,b)}else{const c=await WebAssembly.instantiate(a,b);if(c instanceof WebAssembly.Instance){return {instance:c,module:a}}else{return c}}});var i=((a,b,c)=>{if(c===T){const c=g.encode(a);const e=b(c.length,U)>>>Q;d().subarray(e,e+ c.length).set(c);f=c.length;return e};let e=a.length;let i=b(e,U)>>>Q;const j=d();let k=Q;for(;k<e;k++){const b=a.charCodeAt(k);if(b>127)break;j[i+ k]=b};if(k!==e){if(k!==Q){a=a.slice(k)};i=c(i,e,e=k+ a.length*V,U)>>>Q;const b=d().subarray(i+ k,i+ e);const f=h(a,b);k+=f.written;i=c(i,e,k,U)>>>Q};f=k;return i});function w(b,c){try{return b.apply(this,c)}catch(b){const c=v(b);a.__wbindgen_exn_store(c)}}var J=((b,d)=>{a=b.exports;L.__wbindgen_wasm_module=d;k=P;c=P;a.__wbindgen_start();return a});var I=((a,b)=>{});var q=((b,c)=>{a._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h2a9df5c3aa7f2b7a(b,c)});let a;const b=typeof TextDecoder!==M?new TextDecoder(N,{ignoreBOM:!0,fatal:!0}):{decode:()=>{throw O(`TextDecoder not available`)}};if(typeof TextDecoder!==M){b.decode()};let c=P;let f=Q;const g=typeof TextEncoder!==M?new TextEncoder(N):{encode:()=>{throw O(`TextEncoder not available`)}};const h=typeof g.encodeInto===S?((a,b)=>g.encodeInto(a,b)):((a,b)=>{const c=g.encode(a);b.set(c);return {read:a.length,written:c.length}});let k=P;const n=typeof Z===M?{register:()=>{},unregister:()=>{}}:new Z(b=>{a.__wbindgen_export_3.get(b.dtor)(b.a,b.b)});const z=[`blob`,`arraybuffer`];const A=[``,`no-referrer`,`no-referrer-when-downgrade`,`origin`,`origin-when-cross-origin`,`unsafe-url`,$,`strict-origin`,`strict-origin-when-cross-origin`];const B=[a0,`no-store`,`reload`,`no-cache`,`force-cache`,`only-if-cached`];const C=[`omit`,$,`include`];const D=[$,`no-cors`,a1,`navigate`];const E=[`follow`,a2,`manual`];const F=[`basic`,a1,a0,a2,`opaque`,`opaqueredirect`];export default L;export{K as initSync}