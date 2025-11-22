// This file describes Global Javascript objects and their methods/properties
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects

use lazy_static::lazy_static;
use rand::prelude::IndexedRandom;

use crate::mutators::js_objects::js_types::*;

#[derive(Debug, Clone)]
pub struct JsGlobalObject {
    sym: String,
    methods: Vec<JsMethod>,
    properties: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum JsMethodKind {
    Static,
    Instance
}

#[derive(Debug, Clone)]
pub struct JsMethod {
    sym: String,
    kind: JsMethodKind,
    signatures: Vec<JsMethodSignature>,
}

#[derive(Clone)]
pub struct JsProperty {
    sym: String,
    kind: JsMethodKind
}

#[derive(Debug, Clone)]
pub struct JsMethodSignature {
    types: Vec<JsObjectType>,
}

impl JsMethodSignature {
    fn new(types: &[JsObjectType]) -> Self {
        Self { types: types.to_vec() }
    }

    pub fn types(&self) -> &[JsObjectType] {
        &self.types
    }
}

impl JsGlobalObject {
    fn new(
        sym: &str,
        methods: &[(&str, JsMethodKind, &[&[JsObjectType]])],
        properties: &[(&str, JsMethodKind)],
    ) -> Self {
        Self {
            sym: sym.to_string(),
            methods: methods
                .iter()
                .map(|(sym, kind, signatures)| JsMethod {
                    sym: sym.to_string(),
                    kind: *kind,
                    signatures: JsGlobalObject::build_signatures(signatures),
                })
                .collect(),
            properties: properties.iter().map(|(sym, _)| sym.to_string()).collect(),
        }
    }

    fn build_signatures(signatures: &[&[JsObjectType]]) -> Vec<JsMethodSignature> {
        if signatures.is_empty() {
            return vec![JsMethodSignature::new(&[])];
        }

        signatures.iter().map(|sig| JsMethodSignature::new(sig)).collect()
    }

    pub fn sym(&self) -> &str {
        &self.sym
    }

    pub fn methods(&self) -> &[JsMethod] {
        &self.methods
    }

    pub fn get_constructor_signatures(&self) -> Vec<JsMethodSignature> {
        self.methods
            .iter()
            .find(|method| method.sym() == self.sym())
            .map(|method| method.signatures().to_vec())
            .unwrap_or_default()
    }
}

impl JsMethod {
    pub fn sym(&self) -> &str {
        &self.sym
    }

    pub fn kind(&self) -> JsMethodKind {
        self.kind
    }

    pub fn signatures(&self) -> &[JsMethodSignature] {
        &self.signatures
    }
}

pub fn get_random_global_object(rng: &mut rand::rngs::ThreadRng) -> JsGlobalObject {
    let global_objects = JS_GLOBAL_OBJECTS.clone();
    // global_objects.choose(rng)
    global_objects[0]
        // .expect("should never panic cause of hardcoded array")
        .clone()
}

pub fn get_global_object(sym: &str) -> Option<JsGlobalObject> {
    JS_GLOBAL_OBJECTS.iter().find(|obj| obj.sym == sym).cloned()
}

use JsMethodKind::*;
use JsObjectType::*;

// TODO: maybe I don't want to enforce types in order to explore more code posibilities
lazy_static! {
    static ref JS_GLOBAL_OBJECTS: Vec<JsGlobalObject> = vec![
        JsGlobalObject::new(
            // object name
            "Array",
            // object methods
            &[
                ("Array", Static, &[
                        &[],
                        &[Number],
                        // &[Any],
                        // &[Any, Any],
                        // &[Any, Any, Any],
                    ]),
                ("from", Static, &[
                        &[Any],
                        &[Any, Function],
                        &[Any, Function, Any],
                    ]),
                ("fromAsync", Static, &[
                        &[Any],
                        &[Any, Function],
                        &[Any, Function, Any],
                    ]),
                ("isArray", Static, &[
                        &[Any],
                    ]),
                ("of", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("at", Instance, &[
                        &[Number],
                    ]),
                ("concat", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("copyWithin", Instance, &[
                        &[Number, Number],
                        &[Number, Number, Number],
                    ]),
                ("entries", Instance, &[
                        &[],
                    ]),
                ("every", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("fill", Instance, &[
                        &[Any],
                        &[Any, Number],
                        &[Any, Number, Number],
                    ]),
                ("filter", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("find", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("findIndex", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("findLast", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("findLastIndex", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("flat", Instance, &[
                        &[],
                        &[Number],
                    ]),
                ("flatMap", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("forEach", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("includes", Instance, &[
                        &[Any],
                        &[Any, Number],
                    ]),
                ("indexOf", Instance, &[
                        &[Any],
                        &[Any, Number],
                    ]),
                ("join", Instance, &[
                        &[],
                        &[JsString],
                    ]),
                ("keys", Instance, &[
                        &[],
                    ]),
                ("lastIndexOf", Instance, &[
                        &[Any],
                        &[Any, Number],
                    ]),
                ("map", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("pop", Instance, &[
                        &[],
                    ]),
                ("push", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("reduce", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("reduceRight", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("reverse", Instance, &[
                        &[],
                    ]),
                ("shift", Instance, &[
                        &[],
                    ]),
                ("slice", Instance, &[
                        &[],
                        &[Number],
                        &[Number, Number],
                    ]),
                ("some", Instance, &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("sort", Instance, &[
                        &[],
                        &[Function],
                    ]),
                ("splice", Instance, &[
                        &[Number],
                        &[Number, Number],
                        &[Number, Number, Any],
                        &[Number, Number, Any, Any],
                        &[Number, Number, Any, Any, Any],
                    ]),
                ("toLocaleString", Instance, &[
                        &[],
                        &[JsString],
                        &[JsString, Object],
                    ]),
                ("toReversed", Instance, &[
                        &[],
                    ]),
                ("toSorted", Instance, &[
                        &[],
                        &[Function],
                    ]),
                ("toSpliced", Instance, &[
                        &[Number],
                        &[Number, Number],
                        &[Number, Number, Any],
                        &[Number, Number, Any, Any],
                        &[Number, Number, Any, Any, Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("unshift", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("values", Instance, &[
                        &[],
                    ]),
                ("with", Instance, &[
                        &[Number, Any],
                    ]),
                ("Symbol.iterator", Instance, &[
                        &[],
                    ]),
            ],
            // object properties
            &[
                ("length", Instance),
                ("Symbol.unscopables", Instance),
                ("Symbol.species", Static),
            ],
        ),
        JsGlobalObject::new(
            "ArrayBuffer",
            &[
                ("ArrayBuffer", Static, &[]),
                ("isView", Static, &[
                        &[Any],
                    ]),
                ("resize", Instance, &[
                        &[Any],
                    ]),
                ("slice", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("transfer", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("transferToFixedLength", Instance, &[
                        &[],
                        &[Any],
                    ]),
            ],
            &[
                ("Symbol.species", Static),
                ("byteLength", Instance),
                ("detached", Instance),
                ("maxByteLength", Instance),
                ("resizable", Instance),
            ],
        ),
        JsGlobalObject::new(
            "AsyncDisposableStack",
            &[
                ("AsyncDisposableStack", Static, &[]),
                ("adopt", Instance, &[
                        &[Any, Any],
                    ]),
                ("defer", Instance, &[
                        &[Any],
                    ]),
                ("disposeAsync", Instance, &[
                        &[],
                    ]),
                ("move", Instance, &[
                        &[],
                    ]),
                ("use", Instance, &[
                        &[Any],
                    ]),
                ("Symbol.asyncDispose", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("disposed", Instance),
            ],
        ),
        JsGlobalObject::new(
            "AsyncFunction",
            &[
                ("AsyncFunction", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "AsyncGenerator",
            &[
                ("next", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("return", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("throw", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "AsyncGeneratorFunction",
            &[
                ("AsyncGeneratorFunction", Static, &[]),
            ],
            &[
                ("prototype", Instance),
            ],
        ),
        JsGlobalObject::new(
            "AsyncIterator",
            &[
                ("Symbol.asyncDispose", Instance, &[
                        &[],
                    ]),
                ("Symbol.asyncIterator", Instance, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Atomics",
            &[
                ("add", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("and", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("compareExchange", Static, &[
                        &[Any, Any, Any, Any],
                    ]),
                ("exchange", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("isLockFree", Static, &[
                        &[Any],
                    ]),
                ("load", Static, &[
                        &[Any, Any],
                    ]),
                ("notify", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("or", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("pause", Static, &[
                        &[],
                        &[Any],
                    ]),
                ("store", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("sub", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("wait", Static, &[
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("waitAsync", Static, &[
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("xor", Static, &[
                        &[Any, Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "BigInt",
            &[
                ("BigInt", Static, &[]),
                ("asIntN", Static, &[
                        &[Any, Any],
                    ]),
                ("asUintN", Static, &[
                        &[Any, Any],
                    ]),
                ("toLocaleString", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "BigInt64Array",
            &[
                ("BigInt64Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "BigUint64Array",
            &[
                ("BigUint64Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Boolean",
            &[
                ("Boolean", Static, &[]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "DataView",
            &[
                ("DataView", Static, &[]),
                ("getBigInt64", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getBigUint64", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getFloat16", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getFloat32", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getFloat64", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getInt8", Instance, &[
                        &[Any],
                    ]),
                ("getInt16", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getInt32", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getUint8", Instance, &[
                        &[Any],
                    ]),
                ("getUint16", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getUint32", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setBigInt64", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setBigUint64", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setFloat16", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setFloat32", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setFloat64", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setInt8", Instance, &[
                        &[Any, Any],
                    ]),
                ("setInt16", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setInt32", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUint8", Instance, &[
                        &[Any, Any],
                    ]),
                ("setUint16", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUint32", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
            ],
            &[
                ("buffer", Instance),
                ("byteLength", Instance),
                ("byteOffset", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Date",
            &[
                ("Date", Static, &[]),
                ("now", Static, &[
                        &[],
                    ]),
                ("parse", Static, &[
                        &[Any],
                    ]),
                ("UTC", Static, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                        &[Any, Any, Any, Any, Any],
                        &[Any, Any, Any, Any, Any, Any],
                        &[Any, Any, Any, Any, Any, Any, Any],
                    ]),
                ("getDate", Instance, &[
                        &[],
                    ]),
                ("getDay", Instance, &[
                        &[],
                    ]),
                ("getFullYear", Instance, &[
                        &[],
                    ]),
                ("getHours", Instance, &[
                        &[],
                    ]),
                ("getMilliseconds", Instance, &[
                        &[],
                    ]),
                ("getMinutes", Instance, &[
                        &[],
                    ]),
                ("getMonth", Instance, &[
                        &[],
                    ]),
                ("getSeconds", Instance, &[
                        &[],
                    ]),
                ("getTime", Instance, &[
                        &[],
                    ]),
                ("getTimezoneOffset", Instance, &[
                        &[],
                    ]),
                ("getUTCDate", Instance, &[
                        &[],
                    ]),
                ("getUTCDay", Instance, &[
                        &[],
                    ]),
                ("getUTCFullYear", Instance, &[
                        &[],
                    ]),
                ("getUTCHours", Instance, &[
                        &[],
                    ]),
                ("getUTCMilliseconds", Instance, &[
                        &[],
                    ]),
                ("getUTCMinutes", Instance, &[
                        &[],
                    ]),
                ("getUTCMonth", Instance, &[
                        &[],
                    ]),
                ("getUTCSeconds", Instance, &[
                        &[],
                    ]),
                ("setDate", Instance, &[
                        &[Any],
                    ]),
                ("setFullYear", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setHours", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("setMilliseconds", Instance, &[
                        &[Any],
                    ]),
                ("setMinutes", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setMonth", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setSeconds", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setTime", Instance, &[
                        &[Any],
                    ]),
                ("setUTCDate", Instance, &[
                        &[Any],
                    ]),
                ("setUTCFullYear", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUTCHours", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("setUTCMilliseconds", Instance, &[
                        &[Any],
                    ]),
                ("setUTCMinutes", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUTCMonth", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setUTCSeconds", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toDateString", Instance, &[
                        &[],
                    ]),
                ("toISOString", Instance, &[
                        &[],
                    ]),
                ("toJSON", Instance, &[
                        &[],
                    ]),
                ("toLocaleDateString", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleString", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleTimeString", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("toTimeString", Instance, &[
                        &[],
                    ]),
                ("toUTCString", Instance, &[
                        &[],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
                ("Symbol.toPrimitive", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "DisposableStack",
            &[
                ("DisposableStack", Static, &[]),
                ("adopt", Instance, &[
                        &[Any, Any],
                    ]),
                ("defer", Instance, &[
                        &[Any],
                    ]),
                ("dispose", Instance, &[
                        &[],
                    ]),
                ("move", Instance, &[
                        &[],
                    ]),
                ("use", Instance, &[
                        &[Any],
                    ]),
                ("Symbol.dispose", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("disposed", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Error",
            &[
                ("Error", Static, &[]),
                ("captureStackTrace", Static, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("isError", Static, &[
                        &[Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("cause", Instance),
                ("message", Instance),
                ("name", Instance),
            ],
        ),
        JsGlobalObject::new(
            "EvalError",
            &[
                ("EvalError", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "FinalizationRegistry",
            &[
                ("FinalizationRegistry", Static, &[]),
                ("register", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("unregister", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Float16Array",
            &[
                ("Float16Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Float32Array",
            &[
                ("Float32Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Float64Array",
            &[
                ("Float64Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Function",
            &[
                ("Function", Static, &[]),
                ("apply", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("bind", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("call", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("Symbol.hasInstance", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
                ("length", Instance),
                ("name", Instance),
                ("prototype", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Generator",
            &[
                ("next", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("return", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("throw", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "GeneratorFunction",
            &[
                ("GeneratorFunction", Static, &[]),
            ],
            &[
                ("prototype", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Int8Array",
            &[
                ("Int8Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Int16Array",
            &[
                ("Int16Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Int32Array",
            &[
                ("Int32Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Intl",
            &[
                ("getCanonicalLocales", Static, &[
                        &[Any],
                    ]),
                ("supportedValuesOf", Static, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Iterator",
            &[
                ("Iterator", Static, &[]),
                ("from", Static, &[
                        &[Any],
                    ]),
                ("drop", Instance, &[
                        &[Any],
                    ]),
                ("every", Instance, &[
                        &[Any],
                    ]),
                ("filter", Instance, &[
                        &[Any],
                    ]),
                ("find", Instance, &[
                        &[Any],
                    ]),
                ("flatMap", Instance, &[
                        &[Any],
                    ]),
                ("forEach", Instance, &[
                        &[Any],
                    ]),
                ("map", Instance, &[
                        &[Any],
                    ]),
                ("reduce", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("some", Instance, &[
                        &[Any],
                    ]),
                ("take", Instance, &[
                        &[Any],
                    ]),
                ("toArray", Instance, &[
                        &[],
                    ]),
                ("Symbol.dispose", Instance, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "JSON",
            &[
                ("isRawJSON", Static, &[
                        &[Any],
                    ]),
                ("parse", Static, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("rawJSON", Static, &[
                        &[Any],
                    ]),
                ("stringify", Static, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Map",
            &[
                ("Map", Static, &[]),
                ("groupBy", Static, &[
                        &[Any, Any],
                    ]),
                ("clear", Instance, &[
                        &[],
                    ]),
                ("delete", Instance, &[
                        &[Any],
                    ]),
                ("entries", Instance, &[
                        &[],
                    ]),
                ("forEach", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("get", Instance, &[
                        &[Any],
                    ]),
                ("has", Instance, &[
                        &[Any],
                    ]),
                ("keys", Instance, &[
                        &[],
                    ]),
                ("set", Instance, &[
                        &[Any, Any],
                    ]),
                ("values", Instance, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("Symbol.species", Static),
                ("size", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Math",
            &[
                ("abs", Static, &[
                        &[Any],
                    ]),
                ("acos", Static, &[
                        &[Any],
                    ]),
                ("acosh", Static, &[
                        &[Any],
                    ]),
                ("asin", Static, &[
                        &[Any],
                    ]),
                ("asinh", Static, &[
                        &[Any],
                    ]),
                ("atan", Static, &[
                        &[Any],
                    ]),
                ("atan2", Static, &[
                        &[Any, Any],
                    ]),
                ("atanh", Static, &[
                        &[Any],
                    ]),
                ("cbrt", Static, &[
                        &[Any],
                    ]),
                ("ceil", Static, &[
                        &[Any],
                    ]),
                ("clz32", Static, &[
                        &[Any],
                    ]),
                ("cos", Static, &[
                        &[Any],
                    ]),
                ("cosh", Static, &[
                        &[Any],
                    ]),
                ("exp", Static, &[
                        &[Any],
                    ]),
                ("expm1", Static, &[
                        &[Any],
                    ]),
                ("f16round", Static, &[
                        &[Any],
                    ]),
                ("floor", Static, &[
                        &[Any],
                    ]),
                ("fround", Static, &[
                        &[Any],
                    ]),
                ("hypot", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("imul", Static, &[
                        &[Any, Any],
                    ]),
                ("log", Static, &[
                        &[Any],
                    ]),
                ("log1p", Static, &[
                        &[Any],
                    ]),
                ("log2", Static, &[
                        &[Any],
                    ]),
                ("log10", Static, &[
                        &[Any],
                    ]),
                ("max", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("min", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("pow", Static, &[
                        &[Any, Any],
                    ]),
                ("random", Static, &[
                        &[],
                    ]),
                ("round", Static, &[
                        &[Any],
                    ]),
                ("sign", Static, &[
                        &[Any],
                    ]),
                ("sin", Static, &[
                        &[Any],
                    ]),
                ("sinh", Static, &[
                        &[Any],
                    ]),
                ("sqrt", Static, &[
                        &[Any],
                    ]),
                ("sumPrecise", Static, &[
                        &[Any],
                    ]),
                ("tan", Static, &[
                        &[Any],
                    ]),
                ("tanh", Static, &[
                        &[Any],
                    ]),
                ("trunc", Static, &[
                        &[Any],
                    ]),
            ],
            &[
                ("E", Static),
                ("LN2", Static),
                ("LN10", Static),
                ("LOG2E", Static),
                ("LOG10E", Static),
                ("PI", Static),
                ("SQRT1_2", Static),
                ("SQRT2", Static),
            ],
        ),
        JsGlobalObject::new(
            "Number",
            &[
                ("Number", Static, &[]),
                ("isFinite", Static, &[
                        &[Any],
                    ]),
                ("isInteger", Static, &[
                        &[Any],
                    ]),
                ("isNaN", Static, &[
                        &[Any],
                    ]),
                ("isSafeInteger", Static, &[
                        &[Any],
                    ]),
                ("parseFloat", Static, &[
                        &[Any],
                    ]),
                ("parseInt", Static, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toExponential", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toFixed", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toLocaleString", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toPrecision", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("EPSILON", Static),
                ("MAX_SAFE_INTEGER", Static),
                ("MAX_VALUE", Static),
                ("MIN_SAFE_INTEGER", Static),
                ("MIN_VALUE", Static),
                ("NaN", Static),
                ("NEGATIVE_INFINITY", Static),
                ("POSITIVE_INFINITY", Static),
            ],
        ),
        JsGlobalObject::new(
            "Object",
            &[
                ("Object", Static, &[]),
                ("assign", Static, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("create", Static, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("defineProperties", Static, &[
                        &[Any, Any],
                    ]),
                ("defineProperty", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("entries", Static, &[
                        &[Any],
                    ]),
                ("freeze", Static, &[
                        &[Any],
                    ]),
                ("fromEntries", Static, &[
                        &[Any],
                    ]),
                ("getOwnPropertyDescriptor", Static, &[
                        &[Any, Any],
                    ]),
                ("getOwnPropertyDescriptors", Static, &[
                        &[Any],
                    ]),
                ("getOwnPropertyNames", Static, &[
                        &[Any],
                    ]),
                ("getOwnPropertySymbols", Static, &[
                        &[Any],
                    ]),
                ("getPrototypeOf", Static, &[
                        &[Any],
                    ]),
                ("groupBy", Static, &[
                        &[Any, Any],
                    ]),
                ("hasOwn", Static, &[
                        &[Any, Any],
                    ]),
                ("is", Static, &[
                        &[Any, Any],
                    ]),
                ("isExtensible", Static, &[
                        &[Any],
                    ]),
                ("isFrozen", Static, &[
                        &[Any],
                    ]),
                ("isSealed", Static, &[
                        &[Any],
                    ]),
                ("keys", Static, &[
                        &[Any],
                    ]),
                ("preventExtensions", Static, &[
                        &[Any],
                    ]),
                ("seal", Static, &[
                        &[Any],
                    ]),
                ("setPrototypeOf", Static, &[
                        &[Any, Any],
                    ]),
                ("values", Static, &[
                        &[Any],
                    ]),
                ("hasOwnProperty", Instance, &[
                        &[Any],
                    ]),
                ("isPrototypeOf", Instance, &[
                        &[Any],
                    ]),
                ("propertyIsEnumerable", Instance, &[
                        &[Any],
                    ]),
                ("toLocaleString", Instance, &[
                        &[],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("constructor", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Promise",
            &[
                ("Promise", Static, &[]),
                ("all", Static, &[
                        &[Any],
                    ]),
                ("allSettled", Static, &[
                        &[Any],
                    ]),
                ("any", Static, &[
                        &[Any],
                    ]),
                ("race", Static, &[
                        &[Any],
                    ]),
                ("reject", Static, &[
                        &[Any],
                    ]),
                ("resolve", Static, &[
                        &[Any],
                    ]),
                ("try", Static, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("withResolvers", Static, &[
                        &[],
                    ]),
                ("catch", Instance, &[
                        &[Any],
                    ]),
                ("finally", Instance, &[
                        &[Any],
                    ]),
                ("then", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
            ],
            &[
                ("Symbol.species", Static),
            ],
        ),
        JsGlobalObject::new(
            "Proxy",
            &[
                ("Proxy", Static, &[]),
                ("revocable", Static, &[
                        &[Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "RangeError",
            &[
                ("RangeError", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "ReferenceError",
            &[
                ("ReferenceError", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Reflect",
            &[
                ("apply", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("construct", Static, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("defineProperty", Static, &[
                        &[Any, Any, Any],
                    ]),
                ("deleteProperty", Static, &[
                        &[Any, Any],
                    ]),
                ("get", Static, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("getOwnPropertyDescriptor", Static, &[
                        &[Any, Any],
                    ]),
                ("getPrototypeOf", Static, &[
                        &[Any],
                    ]),
                ("has", Static, &[
                        &[Any, Any],
                    ]),
                ("isExtensible", Static, &[
                        &[Any],
                    ]),
                ("ownKeys", Static, &[
                        &[Any],
                    ]),
                ("preventExtensions", Static, &[
                        &[Any],
                    ]),
                ("set", Static, &[
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("setPrototypeOf", Static, &[
                        &[Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "RegExp",
            &[
                ("RegExp", Static, &[]),
                ("escape", Static, &[
                        &[Any],
                    ]),
                ("exec", Instance, &[
                        &[Any],
                    ]),
                ("test", Instance, &[
                        &[Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("Symbol.match", Instance, &[
                        &[Any],
                    ]),
                ("Symbol.matchAll", Instance, &[
                        &[Any],
                    ]),
                ("Symbol.replace", Instance, &[
                        &[Any, Any],
                    ]),
                ("Symbol.search", Instance, &[
                        &[Any],
                    ]),
                ("Symbol.split", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
            ],
            &[
                ("Symbol.species", Static),
                ("lastIndex", Instance),
                ("dotAll", Instance),
                ("flags", Instance),
                ("global", Instance),
                ("hasIndices", Instance),
                ("ignoreCase", Instance),
                ("multiline", Instance),
                ("source", Instance),
                ("sticky", Instance),
                ("unicode", Instance),
                ("unicodeSets", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Set",
            &[
                ("Set", Static, &[]),
                ("add", Instance, &[
                        &[Any],
                    ]),
                ("clear", Instance, &[
                        &[],
                    ]),
                ("delete", Instance, &[
                        &[Any],
                    ]),
                ("difference", Instance, &[
                        &[Any],
                    ]),
                ("entries", Instance, &[
                        &[],
                    ]),
                ("forEach", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("has", Instance, &[
                        &[Any],
                    ]),
                ("intersection", Instance, &[
                        &[Any],
                    ]),
                ("isDisjointFrom", Instance, &[
                        &[Any],
                    ]),
                ("isSubsetOf", Instance, &[
                        &[Any],
                    ]),
                ("isSupersetOf", Instance, &[
                        &[Any],
                    ]),
                ("keys", Instance, &[
                        &[],
                    ]),
                ("symmetricDifference", Instance, &[
                        &[Any],
                    ]),
                ("union", Instance, &[
                        &[Any],
                    ]),
                ("values", Instance, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("Symbol.species", Static),
                ("size", Instance),
            ],
        ),
        JsGlobalObject::new(
            "SharedArrayBuffer",
            &[
                ("SharedArrayBuffer", Static, &[]),
                ("grow", Instance, &[
                        &[Any],
                    ]),
                ("slice", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
            ],
            &[
                ("Symbol.species", Static),
                ("byteLength", Instance),
                ("growable", Instance),
                ("maxByteLength", Instance),
            ],
        ),
        JsGlobalObject::new(
            "String",
            &[
                ("String", Static, &[]),
                ("fromCharCode", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("fromCodePoint", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("raw", Static, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                        &[],
                    ]),
                ("at", Instance, &[
                        &[Any],
                    ]),
                ("charAt", Instance, &[
                        &[Any],
                    ]),
                ("charCodeAt", Instance, &[
                        &[Any],
                    ]),
                ("codePointAt", Instance, &[
                        &[Any],
                    ]),
                ("concat", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("endsWith", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("includes", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("indexOf", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("isWellFormed", Instance, &[
                        &[],
                    ]),
                ("lastIndexOf", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("localeCompare", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("match", Instance, &[
                        &[Any],
                    ]),
                ("matchAll", Instance, &[
                        &[Any],
                    ]),
                ("normalize", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("padEnd", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("padStart", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("repeat", Instance, &[
                        &[Any],
                    ]),
                ("replace", Instance, &[
                        &[Any, Any],
                    ]),
                ("replaceAll", Instance, &[
                        &[Any, Any],
                    ]),
                ("search", Instance, &[
                        &[Any],
                    ]),
                ("slice", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("split", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("startsWith", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("substring", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleLowerCase", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toLocaleUpperCase", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toLowerCase", Instance, &[
                        &[],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("toUpperCase", Instance, &[
                        &[],
                    ]),
                ("toWellFormed", Instance, &[
                        &[],
                    ]),
                ("trim", Instance, &[
                        &[],
                    ]),
                ("trimEnd", Instance, &[
                        &[],
                    ]),
                ("trimStart", Instance, &[
                        &[],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("length", Instance),
            ],
        ),
        JsGlobalObject::new(
            "SuppressedError",
            &[
                ("SuppressedError", Static, &[]),
            ],
            &[
                ("error", Instance),
                ("suppressed", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Symbol",
            &[
                ("Symbol", Static, &[]),
                ("for", Static, &[
                        &[Any],
                    ]),
                ("keyFor", Static, &[
                        &[Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("valueOf", Instance, &[
                        &[],
                    ]),
                ("Symbol.toPrimitive", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
                ("asyncDispose", Static),
                ("asyncIterator", Static),
                ("dispose", Static),
                ("hasInstance", Static),
                ("isConcatSpreadable", Static),
                ("iterator", Static),
                ("match", Static),
                ("matchAll", Static),
                ("replace", Static),
                ("search", Static),
                ("species", Static),
                ("split", Static),
                ("toPrimitive", Static),
                ("toStringTag", Static),
                ("unscopables", Static),
                ("description", Instance),
            ],
        ),
        JsGlobalObject::new(
            "SyntaxError",
            &[
                ("SyntaxError", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "TypedArray",
            &[
                ("from", Static, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("of", Static, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("at", Instance, &[
                        &[Any],
                    ]),
                ("copyWithin", Instance, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("entries", Instance, &[
                        &[],
                    ]),
                ("every", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("fill", Instance, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("filter", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("find", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("findIndex", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("findLast", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("findLastIndex", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("forEach", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("includes", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("indexOf", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("join", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("keys", Instance, &[
                        &[],
                    ]),
                ("lastIndexOf", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("map", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("reduce", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("reduceRight", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("reverse", Instance, &[
                        &[],
                    ]),
                ("set", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("slice", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("some", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("sort", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("subarray", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleString", Instance, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toReversed", Instance, &[
                        &[],
                    ]),
                ("toSorted", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toString", Instance, &[
                        &[],
                    ]),
                ("values", Instance, &[
                        &[],
                    ]),
                ("with", Instance, &[
                        &[Any, Any],
                    ]),
                ("Symbol.iterator", Instance, &[
                        &[],
                    ]),
            ],
            &[
                ("BYTES_PER_ELEMENT", Static),
                ("Symbol.species", Static),
                ("buffer", Instance),
                ("byteLength", Instance),
                ("byteOffset", Instance),
                ("length", Instance),
            ],
        ),
        JsGlobalObject::new(
            "TypeError",
            &[
                ("TypeError", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint8Array",
            &[
                ("Uint8Array", Static, &[]),
                ("fromBase64", Static, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("fromHex", Static, &[
                        &[Any],
                    ]),
                ("setFromBase64", Instance, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setFromHex", Instance, &[
                        &[Any],
                    ]),
                ("toBase64", Instance, &[
                        &[],
                        &[Any],
                    ]),
                ("toHex", Instance, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint8ClampedArray",
            &[
                ("Uint8ClampedArray", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint16Array",
            &[
                ("Uint16Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint32Array",
            &[
                ("Uint32Array", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "URIError",
            &[
                ("URIError", Static, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "WeakMap",
            &[
                ("WeakMap", Static, &[]),
                ("delete", Instance, &[
                        &[Any],
                    ]),
                ("get", Instance, &[
                        &[Any],
                    ]),
                ("has", Instance, &[
                        &[Any],
                    ]),
                ("set", Instance, &[
                        &[Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "WeakRef",
            &[
                ("WeakRef", Static, &[]),
                ("deref", Instance, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "WeakSet",
            &[
                ("WeakSet", Static, &[]),
                ("add", Instance, &[
                        &[Any],
                    ]),
                ("delete", Instance, &[
                        &[Any],
                    ]),
                ("has", Instance, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
    ];
}
