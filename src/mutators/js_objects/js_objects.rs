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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsMethodKind {
    Static,
    Instance
}

#[derive(Debug, Clone)]
pub struct JsMethod {
    sym: String,
    kind: JsMethodKind,
    signatures: Vec<JsMethodSignature>,
    returns: Option<JsObjectType>,
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
        methods: &[(&str, JsMethodKind, Option<JsObjectType>, &[&[JsObjectType]])],
        properties: &[(&str, JsMethodKind)],
    ) -> Self {
        Self {
            sym: sym.to_string(),
            methods: methods
                .iter()
                .map(|(sym, kind, returns, signatures)| JsMethod {
                    sym: sym.to_string(),
                    kind: *kind,
                    signatures: JsGlobalObject::build_signatures(signatures),
                    returns: *returns,
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

    pub fn instance_methods(&self) -> Vec<&JsMethod> {
        self.methods
            .iter()
            .filter(|method| method.kind() == JsMethodKind::Instance)
            .collect()
    }

    pub fn get_constructor_signatures(&self) -> Vec<JsMethodSignature> {
        self.methods
            .iter()
            .find(|method| method.sym() == self.sym())
            .map(|method| method.signatures().to_vec())
            .unwrap_or_default()
    }

    pub fn static_methods(&self) -> Vec<&JsMethod> {
        self.methods
            .iter()
            .filter(|method| method.kind() == JsMethodKind::Static)
            .collect()
    }

    pub fn from_js_type(ty: JsObjectType) -> JsGlobalObject {
        match ty {
            JsObjectType::Array => get_global_object("Array").unwrap(),
            JsObjectType::Boolean => get_global_object("Boolean").unwrap(),
            JsObjectType::Number => get_global_object("Number").unwrap(),
            JsObjectType::JsString => get_global_object("String").unwrap(),
            JsObjectType::Object => get_global_object("Object").unwrap(),
            _ => panic!("No global object for type {:?}", ty),
        }
    }

    pub fn to_js_type(&self) -> JsObjectType {
        match self.sym.as_str() {
            "Array" => JsObjectType::Array,
            "Boolean" => JsObjectType::Boolean,
            "Number" => JsObjectType::Number,
            "String" => JsObjectType::JsString,
            "Object" => JsObjectType::Object,
            _ => JsObjectType::Object, // default to Object for unknown types
        }
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

    pub fn returns(&self) -> Option<JsObjectType> {
        self.returns
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
                ("Array", Static, Some(Array), &[
                        &[],
                        &[Number],
                        // &[Any],
                        // &[Any, Any],
                        // &[Any, Any, Any],
                    ]),
                ("from", Static, Some(Array), &[
                        &[Any],
                        &[Any, Function],
                        &[Any, Function, Any],
                    ]),
                ("fromAsync", Static, Some(Array), &[
                        &[Any],
                        &[Any, Function],
                        &[Any, Function, Any],
                    ]),
                ("isArray", Static, Some(Boolean), &[
                        &[Any],
                    ]),
                ("of", Static, Some(Array), &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("at", Instance, Some(Any), &[
                        &[Number],
                    ]),
                ("concat", Instance, Some(Array), &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("copyWithin", Instance, Some(Array), &[
                        &[Number, Number],
                        &[Number, Number, Number],
                    ]),
                ("entries", Instance, Some(Object), &[
                        &[],
                    ]),
                ("every", Instance, Some(Boolean), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("fill", Instance, Some(Array), &[
                        &[Any],
                        &[Any, Number],
                        &[Any, Number, Number],
                    ]),
                ("filter", Instance, Some(Array), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("find", Instance, Some(Any), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("findIndex", Instance, Some(Number), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("findLast", Instance, Some(Any), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("findLastIndex", Instance, Some(Number), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("flat", Instance, Some(Array), &[
                        &[],
                        &[Number],
                    ]),
                ("flatMap", Instance, Some(Array), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("forEach", Instance, Some(Any), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("includes", Instance, Some(Boolean), &[
                        &[Any],
                        &[Any, Number],
                    ]),
                ("indexOf", Instance, Some(Number), &[
                        &[Any],
                        &[Any, Number],
                    ]),
                ("join", Instance, Some(JsString), &[
                        &[],
                        &[JsString],
                    ]),
                ("keys", Instance, Some(Object), &[
                        &[],
                    ]),
                ("lastIndexOf", Instance, Some(Number), &[
                        &[Any],
                        &[Any, Number],
                    ]),
                ("map", Instance, Some(Array), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("pop", Instance, Some(Any), &[
                        &[],
                    ]),
                ("push", Instance, None, &[
                        // &[],
                        &[Any],
                        // &[Any, Any],
                        // &[Any, Any, Any],
                    ]),
                ("reduce", Instance, Some(Any), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("reduceRight", Instance, Some(Any), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("reverse", Instance, Some(Array), &[
                        &[],
                    ]),
                ("shift", Instance, Some(Any), &[
                        &[],
                    ]),
                ("slice", Instance, Some(Array), &[
                        &[],
                        &[Number],
                        &[Number, Number],
                    ]),
                ("some", Instance, Some(Boolean), &[
                        &[Function],
                        &[Function, Any],
                    ]),
                ("sort", Instance, None, &[
                        &[],
                        &[Function],
                    ]),
                ("splice", Instance, Some(Array), &[
                        &[Number],
                        &[Number, Number],
                        &[Number, Number, Any],
                        &[Number, Number, Any, Any],
                        &[Number, Number, Any, Any, Any],
                    ]),
                ("toLocaleString", Instance, Some(JsString), &[
                        &[],
                        &[JsString],
                        &[JsString, Object],
                    ]),
                ("toReversed", Instance, Some(Array), &[
                        &[],
                    ]),
                ("toSorted", Instance, Some(Array), &[
                        &[],
                        &[Function],
                    ]),
                ("toSpliced", Instance, Some(Array), &[
                        &[Number],
                        &[Number, Number],
                        &[Number, Number, Any],
                        &[Number, Number, Any, Any],
                        &[Number, Number, Any, Any, Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("unshift", Instance, Some(Number), &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("values", Instance, Some(Object), &[
                        &[],
                    ]),
                ("with", Instance, Some(Array), &[
                        &[Number, Any],
                    ]),
                ("Symbol.iterator", Instance, Some(Object), &[
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
                ("ArrayBuffer", Static, None, &[]),
                ("isView", Static, None, &[
                        &[Any],
                    ]),
                ("resize", Instance, None, &[
                        &[Any],
                    ]),
                ("slice", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("transfer", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("transferToFixedLength", Instance, None, &[
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
                ("AsyncDisposableStack", Static, None, &[]),
                ("adopt", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("defer", Instance, None, &[
                        &[Any],
                    ]),
                ("disposeAsync", Instance, None, &[
                        &[],
                    ]),
                ("move", Instance, None, &[
                        &[],
                    ]),
                ("use", Instance, None, &[
                        &[Any],
                    ]),
                ("Symbol.asyncDispose", Instance, None, &[
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
                ("AsyncFunction", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "AsyncGenerator",
            &[
                ("next", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("return", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("throw", Instance, None, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "AsyncGeneratorFunction",
            &[
                ("AsyncGeneratorFunction", Static, None, &[]),
            ],
            &[
                ("prototype", Instance),
            ],
        ),
        JsGlobalObject::new(
            "AsyncIterator",
            &[
                ("Symbol.asyncDispose", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.asyncIterator", Instance, None, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Atomics",
            &[
                ("add", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("and", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("compareExchange", Static, None, &[
                        &[Any, Any, Any, Any],
                    ]),
                ("exchange", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("isLockFree", Static, None, &[
                        &[Any],
                    ]),
                ("load", Static, None, &[
                        &[Any, Any],
                    ]),
                ("notify", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("or", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("pause", Static, None, &[
                        &[],
                        &[Any],
                    ]),
                ("store", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("sub", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("wait", Static, None, &[
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("waitAsync", Static, None, &[
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("xor", Static, None, &[
                        &[Any, Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "BigInt",
            &[
                ("BigInt", Static, None, &[]),
                ("asIntN", Static, None, &[
                        &[Any, Any],
                    ]),
                ("asUintN", Static, None, &[
                        &[Any, Any],
                    ]),
                ("toLocaleString", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                        &[Any],
                    ]),
                ("valueOf", Instance, None, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "BigInt64Array",
            &[
                ("BigInt64Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "BigUint64Array",
            &[
                ("BigUint64Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Boolean",
            &[
                ("Boolean", Static, None, &[]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("valueOf", Instance, None, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "DataView",
            &[
                ("DataView", Static, None, &[]),
                ("getBigInt64", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getBigUint64", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getFloat16", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getFloat32", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getFloat64", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getInt8", Instance, None, &[
                        &[Any],
                    ]),
                ("getInt16", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getInt32", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getUint8", Instance, None, &[
                        &[Any],
                    ]),
                ("getUint16", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("getUint32", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setBigInt64", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setBigUint64", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setFloat16", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setFloat32", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setFloat64", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setInt8", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("setInt16", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setInt32", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUint8", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("setUint16", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUint32", Instance, None, &[
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
                ("Date", Static, None, &[]),
                ("now", Static, None, &[
                        &[],
                    ]),
                ("parse", Static, None, &[
                        &[Any],
                    ]),
                ("UTC", Static, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                        &[Any, Any, Any, Any, Any],
                        &[Any, Any, Any, Any, Any, Any],
                        &[Any, Any, Any, Any, Any, Any, Any],
                    ]),
                ("getDate", Instance, None, &[
                        &[],
                    ]),
                ("getDay", Instance, None, &[
                        &[],
                    ]),
                ("getFullYear", Instance, None, &[
                        &[],
                    ]),
                ("getHours", Instance, None, &[
                        &[],
                    ]),
                ("getMilliseconds", Instance, None, &[
                        &[],
                    ]),
                ("getMinutes", Instance, None, &[
                        &[],
                    ]),
                ("getMonth", Instance, None, &[
                        &[],
                    ]),
                ("getSeconds", Instance, None, &[
                        &[],
                    ]),
                ("getTime", Instance, None, &[
                        &[],
                    ]),
                ("getTimezoneOffset", Instance, None, &[
                        &[],
                    ]),
                ("getUTCDate", Instance, None, &[
                        &[],
                    ]),
                ("getUTCDay", Instance, None, &[
                        &[],
                    ]),
                ("getUTCFullYear", Instance, None, &[
                        &[],
                    ]),
                ("getUTCHours", Instance, None, &[
                        &[],
                    ]),
                ("getUTCMilliseconds", Instance, None, &[
                        &[],
                    ]),
                ("getUTCMinutes", Instance, None, &[
                        &[],
                    ]),
                ("getUTCMonth", Instance, None, &[
                        &[],
                    ]),
                ("getUTCSeconds", Instance, None, &[
                        &[],
                    ]),
                ("setDate", Instance, None, &[
                        &[Any],
                    ]),
                ("setFullYear", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setHours", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("setMilliseconds", Instance, None, &[
                        &[Any],
                    ]),
                ("setMinutes", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setMonth", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setSeconds", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setTime", Instance, None, &[
                        &[Any],
                    ]),
                ("setUTCDate", Instance, None, &[
                        &[Any],
                    ]),
                ("setUTCFullYear", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUTCHours", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("setUTCMilliseconds", Instance, None, &[
                        &[Any],
                    ]),
                ("setUTCMinutes", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("setUTCMonth", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setUTCSeconds", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toDateString", Instance, None, &[
                        &[],
                    ]),
                ("toISOString", Instance, None, &[
                        &[],
                    ]),
                ("toJSON", Instance, None, &[
                        &[],
                    ]),
                ("toLocaleDateString", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleString", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleTimeString", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("toTimeString", Instance, None, &[
                        &[],
                    ]),
                ("toUTCString", Instance, None, &[
                        &[],
                    ]),
                ("valueOf", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.toPrimitive", Instance, None, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "DisposableStack",
            &[
                ("DisposableStack", Static, None, &[]),
                ("adopt", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("defer", Instance, None, &[
                        &[Any],
                    ]),
                ("dispose", Instance, None, &[
                        &[],
                    ]),
                ("move", Instance, None, &[
                        &[],
                    ]),
                ("use", Instance, None, &[
                        &[Any],
                    ]),
                ("Symbol.dispose", Instance, None, &[
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
                ("Error", Static, None, &[]),
                ("captureStackTrace", Static, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("isError", Static, None, &[
                        &[Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
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
                ("EvalError", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "FinalizationRegistry",
            &[
                ("FinalizationRegistry", Static, None, &[]),
                ("register", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("unregister", Instance, None, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Float16Array",
            &[
                ("Float16Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Float32Array",
            &[
                ("Float32Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Float64Array",
            &[
                ("Float64Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Function",
            &[
                ("Function", Static, None, &[]),
                ("apply", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("bind", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("call", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("Symbol.hasInstance", Instance, None, &[
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
                ("next", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("return", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("throw", Instance, None, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "GeneratorFunction",
            &[
                ("GeneratorFunction", Static, None, &[]),
            ],
            &[
                ("prototype", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Int8Array",
            &[
                ("Int8Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Int16Array",
            &[
                ("Int16Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Int32Array",
            &[
                ("Int32Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Intl",
            &[
                ("getCanonicalLocales", Static, None, &[
                        &[Any],
                    ]),
                ("supportedValuesOf", Static, None, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Iterator",
            &[
                ("Iterator", Static, None, &[]),
                ("from", Static, None, &[
                        &[Any],
                    ]),
                ("drop", Instance, None, &[
                        &[Any],
                    ]),
                ("every", Instance, None, &[
                        &[Any],
                    ]),
                ("filter", Instance, None, &[
                        &[Any],
                    ]),
                ("find", Instance, None, &[
                        &[Any],
                    ]),
                ("flatMap", Instance, None, &[
                        &[Any],
                    ]),
                ("forEach", Instance, None, &[
                        &[Any],
                    ]),
                ("map", Instance, None, &[
                        &[Function],
                    ]),
                ("reduce", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("some", Instance, None, &[
                        &[Any],
                    ]),
                ("take", Instance, None, &[
                        &[Any],
                    ]),
                ("toArray", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.dispose", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, None, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "JSON",
            &[
                ("isRawJSON", Static, None, &[
                        &[Any],
                    ]),
                ("parse", Static, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("rawJSON", Static, None, &[
                        &[Any],
                    ]),
                ("stringify", Static, None, &[
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
                ("Map", Static, None, &[]),
                ("groupBy", Static, None, &[
                        &[Any, Any],
                    ]),
                ("clear", Instance, None, &[
                        &[],
                    ]),
                ("delete", Instance, None, &[
                        &[Any],
                    ]),
                ("entries", Instance, None, &[
                        &[],
                    ]),
                ("forEach", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("get", Instance, None, &[
                        &[Any],
                    ]),
                ("has", Instance, None, &[
                        &[Any],
                    ]),
                ("keys", Instance, None, &[
                        &[],
                    ]),
                ("set", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("values", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, None, &[
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
                ("abs", Static, None, &[
                        &[Any],
                    ]),
                ("acos", Static, None, &[
                        &[Any],
                    ]),
                ("acosh", Static, None, &[
                        &[Any],
                    ]),
                ("asin", Static, None, &[
                        &[Any],
                    ]),
                ("asinh", Static, None, &[
                        &[Any],
                    ]),
                ("atan", Static, None, &[
                        &[Any],
                    ]),
                ("atan2", Static, None, &[
                        &[Any, Any],
                    ]),
                ("atanh", Static, None, &[
                        &[Any],
                    ]),
                ("cbrt", Static, None, &[
                        &[Any],
                    ]),
                ("ceil", Static, None, &[
                        &[Any],
                    ]),
                ("clz32", Static, None, &[
                        &[Any],
                    ]),
                ("cos", Static, None, &[
                        &[Any],
                    ]),
                ("cosh", Static, None, &[
                        &[Any],
                    ]),
                ("exp", Static, None, &[
                        &[Any],
                    ]),
                ("expm1", Static, None, &[
                        &[Any],
                    ]),
                ("f16round", Static, None, &[
                        &[Any],
                    ]),
                ("floor", Static, None, &[
                        &[Any],
                    ]),
                ("fround", Static, None, &[
                        &[Any],
                    ]),
                ("hypot", Static, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("imul", Static, None, &[
                        &[Any, Any],
                    ]),
                ("log", Static, None, &[
                        &[Any],
                    ]),
                ("log1p", Static, None, &[
                        &[Any],
                    ]),
                ("log2", Static, None, &[
                        &[Any],
                    ]),
                ("log10", Static, None, &[
                        &[Any],
                    ]),
                ("max", Static, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("min", Static, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("pow", Static, None, &[
                        &[Any, Any],
                    ]),
                ("random", Static, None, &[
                        &[],
                    ]),
                ("round", Static, None, &[
                        &[Any],
                    ]),
                ("sign", Static, None, &[
                        &[Any],
                    ]),
                ("sin", Static, None, &[
                        &[Any],
                    ]),
                ("sinh", Static, None, &[
                        &[Any],
                    ]),
                ("sqrt", Static, None, &[
                        &[Any],
                    ]),
                ("sumPrecise", Static, None, &[
                        &[Any],
                    ]),
                ("tan", Static, None, &[
                        &[Any],
                    ]),
                ("tanh", Static, None, &[
                        &[Any],
                    ]),
                ("trunc", Static, None, &[
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
                ("Number", Static, None, &[]),
                ("isFinite", Static, None, &[
                        &[Any],
                    ]),
                ("isInteger", Static, None, &[
                        &[Any],
                    ]),
                ("isNaN", Static, None, &[
                        &[Any],
                    ]),
                ("isSafeInteger", Static, None, &[
                        &[Any],
                    ]),
                ("parseFloat", Static, None, &[
                        &[Any],
                    ]),
                ("parseInt", Static, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toExponential", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toFixed", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toLocaleString", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toPrecision", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                        &[Any],
                    ]),
                ("valueOf", Instance, None, &[
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
                ("Object", Static, None, &[]),
                ("assign", Static, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("create", Static, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("defineProperties", Static, None, &[
                        &[Any, Any],
                    ]),
                ("defineProperty", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("entries", Static, None, &[
                        &[Any],
                    ]),
                ("freeze", Static, None, &[
                        &[Any],
                    ]),
                ("fromEntries", Static, None, &[
                        &[Any],
                    ]),
                ("getOwnPropertyDescriptor", Static, None, &[
                        &[Any, Any],
                    ]),
                ("getOwnPropertyDescriptors", Static, None, &[
                        &[Any],
                    ]),
                ("getOwnPropertyNames", Static, None, &[
                        &[Any],
                    ]),
                ("getOwnPropertySymbols", Static, None, &[
                        &[Any],
                    ]),
                ("getPrototypeOf", Static, None, &[
                        &[Any],
                    ]),
                ("groupBy", Static, None, &[
                        &[Any, Any],
                    ]),
                ("hasOwn", Static, None, &[
                        &[Any, Any],
                    ]),
                ("is", Static, None, &[
                        &[Any, Any],
                    ]),
                ("isExtensible", Static, None, &[
                        &[Any],
                    ]),
                ("isFrozen", Static, None, &[
                        &[Any],
                    ]),
                ("isSealed", Static, None, &[
                        &[Any],
                    ]),
                ("keys", Static, None, &[
                        &[Any],
                    ]),
                ("preventExtensions", Static, None, &[
                        &[Any],
                    ]),
                ("seal", Static, None, &[
                        &[Any],
                    ]),
                ("setPrototypeOf", Static, None, &[
                        &[Any, Any],
                    ]),
                ("values", Static, None, &[
                        &[Any],
                    ]),
                ("hasOwnProperty", Instance, None, &[
                        &[Any],
                    ]),
                ("isPrototypeOf", Instance, None, &[
                        &[Any],
                    ]),
                ("propertyIsEnumerable", Instance, None, &[
                        &[Any],
                    ]),
                ("toLocaleString", Instance, None, &[
                        &[],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("valueOf", Instance, None, &[
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
                ("Promise", Static, None, &[]),
                ("all", Static, None, &[
                        &[Any],
                    ]),
                ("allSettled", Static, None, &[
                        &[Any],
                    ]),
                ("any", Static, None, &[
                        &[Any],
                    ]),
                ("race", Static, None, &[
                        &[Any],
                    ]),
                ("reject", Static, None, &[
                        &[Any],
                    ]),
                ("resolve", Static, None, &[
                        &[Any],
                    ]),
                ("try", Static, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("withResolvers", Static, None, &[
                        &[],
                    ]),
                ("catch", Instance, None, &[
                        &[Any],
                    ]),
                ("finally", Instance, None, &[
                        &[Any],
                    ]),
                ("then", Instance, None, &[
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
                ("Proxy", Static, None, &[]),
                ("revocable", Static, None, &[
                        &[Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "RangeError",
            &[
                ("RangeError", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "ReferenceError",
            &[
                ("ReferenceError", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Reflect",
            &[
                ("apply", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("construct", Static, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("defineProperty", Static, None, &[
                        &[Any, Any, Any],
                    ]),
                ("deleteProperty", Static, None, &[
                        &[Any, Any],
                    ]),
                ("get", Static, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("getOwnPropertyDescriptor", Static, None, &[
                        &[Any, Any],
                    ]),
                ("getPrototypeOf", Static, None, &[
                        &[Any],
                    ]),
                ("has", Static, None, &[
                        &[Any, Any],
                    ]),
                ("isExtensible", Static, None, &[
                        &[Any],
                    ]),
                ("ownKeys", Static, None, &[
                        &[Any],
                    ]),
                ("preventExtensions", Static, None, &[
                        &[Any],
                    ]),
                ("set", Static, None, &[
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                    ]),
                ("setPrototypeOf", Static, None, &[
                        &[Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "RegExp",
            &[
                ("RegExp", Static, None, &[]),
                ("escape", Static, None, &[
                        &[Any],
                    ]),
                ("exec", Instance, None, &[
                        &[Any],
                    ]),
                ("test", Instance, None, &[
                        &[Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("Symbol.match", Instance, None, &[
                        &[Any],
                    ]),
                ("Symbol.matchAll", Instance, None, &[
                        &[Any],
                    ]),
                ("Symbol.replace", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("Symbol.search", Instance, None, &[
                        &[Any],
                    ]),
                ("Symbol.split", Instance, None, &[
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
                ("Set", Static, None, &[]),
                ("add", Instance, None, &[
                        &[Any],
                    ]),
                ("clear", Instance, None, &[
                        &[],
                    ]),
                ("delete", Instance, None, &[
                        &[Any],
                    ]),
                ("difference", Instance, None, &[
                        &[Any],
                    ]),
                ("entries", Instance, None, &[
                        &[],
                    ]),
                ("forEach", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("has", Instance, None, &[
                        &[Any],
                    ]),
                ("intersection", Instance, None, &[
                        &[Any],
                    ]),
                ("isDisjointFrom", Instance, None, &[
                        &[Any],
                    ]),
                ("isSubsetOf", Instance, None, &[
                        &[Any],
                    ]),
                ("isSupersetOf", Instance, None, &[
                        &[Any],
                    ]),
                ("keys", Instance, None, &[
                        &[],
                    ]),
                ("symmetricDifference", Instance, None, &[
                        &[Any],
                    ]),
                ("union", Instance, None, &[
                        &[Any],
                    ]),
                ("values", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, None, &[
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
                ("SharedArrayBuffer", Static, None, &[]),
                ("grow", Instance, None, &[
                        &[Any],
                    ]),
                ("slice", Instance, None, &[
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
                ("String", Static, None, &[]),
                ("fromCharCode", Static, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("fromCodePoint", Static, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("raw", Static, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                        &[Any, Any, Any, Any],
                        &[],
                    ]),
                ("at", Instance, None, &[
                        &[Any],
                    ]),
                ("charAt", Instance, None, &[
                        &[Any],
                    ]),
                ("charCodeAt", Instance, None, &[
                        &[Any],
                    ]),
                ("codePointAt", Instance, None, &[
                        &[Any],
                    ]),
                ("concat", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("endsWith", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("includes", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("indexOf", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("isWellFormed", Instance, None, &[
                        &[],
                    ]),
                ("lastIndexOf", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("localeCompare", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("match", Instance, None, &[
                        &[Any],
                    ]),
                ("matchAll", Instance, None, &[
                        &[Any],
                    ]),
                ("normalize", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("padEnd", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("padStart", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("repeat", Instance, None, &[
                        &[Any],
                    ]),
                ("replace", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("replaceAll", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("search", Instance, None, &[
                        &[Any],
                    ]),
                ("slice", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("split", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("startsWith", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("substring", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleLowerCase", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toLocaleUpperCase", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toLowerCase", Instance, None, &[
                        &[],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("toUpperCase", Instance, None, &[
                        &[],
                    ]),
                ("toWellFormed", Instance, None, &[
                        &[],
                    ]),
                ("trim", Instance, None, &[
                        &[],
                    ]),
                ("trimEnd", Instance, None, &[
                        &[],
                    ]),
                ("trimStart", Instance, None, &[
                        &[],
                    ]),
                ("valueOf", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.iterator", Instance, None, &[
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
                ("SuppressedError", Static, None, &[]),
            ],
            &[
                ("error", Instance),
                ("suppressed", Instance),
            ],
        ),
        JsGlobalObject::new(
            "Symbol",
            &[
                ("Symbol", Static, None, &[]),
                ("for", Static, None, &[
                        &[Any],
                    ]),
                ("keyFor", Static, None, &[
                        &[Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("valueOf", Instance, None, &[
                        &[],
                    ]),
                ("Symbol.toPrimitive", Instance, None, &[
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
                ("SyntaxError", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "TypedArray",
            &[
                ("from", Static, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("of", Static, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("at", Instance, None, &[
                        &[Any],
                    ]),
                ("copyWithin", Instance, None, &[
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("entries", Instance, None, &[
                        &[],
                    ]),
                ("every", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("fill", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                        &[Any, Any, Any],
                    ]),
                ("filter", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("find", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("findIndex", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("findLast", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("findLastIndex", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("forEach", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("includes", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("indexOf", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("join", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("keys", Instance, None, &[
                        &[],
                    ]),
                ("lastIndexOf", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("map", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("reduce", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("reduceRight", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("reverse", Instance, None, &[
                        &[],
                    ]),
                ("set", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("slice", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("some", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("sort", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("subarray", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toLocaleString", Instance, None, &[
                        &[],
                        &[Any],
                        &[Any, Any],
                    ]),
                ("toReversed", Instance, None, &[
                        &[],
                    ]),
                ("toSorted", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toString", Instance, Some(JsString), &[
                        &[],
                    ]),
                ("values", Instance, None, &[
                        &[],
                    ]),
                ("with", Instance, None, &[
                        &[Any, Any],
                    ]),
                ("Symbol.iterator", Instance, None, &[
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
                ("TypeError", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint8Array",
            &[
                ("Uint8Array", Static, None, &[]),
                ("fromBase64", Static, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("fromHex", Static, None, &[
                        &[Any],
                    ]),
                ("setFromBase64", Instance, None, &[
                        &[Any],
                        &[Any, Any],
                    ]),
                ("setFromHex", Instance, None, &[
                        &[Any],
                    ]),
                ("toBase64", Instance, None, &[
                        &[],
                        &[Any],
                    ]),
                ("toHex", Instance, None, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint8ClampedArray",
            &[
                ("Uint8ClampedArray", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint16Array",
            &[
                ("Uint16Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "Uint32Array",
            &[
                ("Uint32Array", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "URIError",
            &[
                ("URIError", Static, None, &[]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "WeakMap",
            &[
                ("WeakMap", Static, None, &[]),
                ("delete", Instance, None, &[
                        &[Any],
                    ]),
                ("get", Instance, None, &[
                        &[Any],
                    ]),
                ("has", Instance, None, &[
                        &[Any],
                    ]),
                ("set", Instance, None, &[
                        &[Any, Any],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "WeakRef",
            &[
                ("WeakRef", Static, None, &[]),
                ("deref", Instance, None, &[
                        &[],
                    ]),
            ],
            &[
            ],
        ),
        JsGlobalObject::new(
            "WeakSet",
            &[
                ("WeakSet", Static, None, &[]),
                ("add", Instance, None, &[
                        &[Any],
                    ]),
                ("delete", Instance, None, &[
                        &[Any],
                    ]),
                ("has", Instance, None, &[
                        &[Any],
                    ]),
            ],
            &[
            ],
        ),
    ];
}
