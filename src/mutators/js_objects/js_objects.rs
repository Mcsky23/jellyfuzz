// This file describes Global Javascript objects and their methods/properties
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects

use lazy_static::lazy_static;

use crate::mutators::js_objects::js_types::*;

pub struct JsGlobalObject {
    sym: String,
    methods: Vec<JsMethod>,
    properties: Vec<String>,
}

#[derive(Clone, Copy)]
enum JsMethodKind {
    Static,
    Instance
}

pub struct JsMethod {
    sym: String,
    kind: JsMethodKind,
    signatures: Vec<JsMethodSignature>,
}

pub struct JsProperty {
    sym: String,
    kind: JsMethodKind
}

pub struct JsMethodSignature {
    types: Vec<JsObjectType>,
}

impl JsMethodSignature {
    fn new(types: &[JsObjectType]) -> Self {
        Self { types: types.to_vec() }
    }
}

impl JsGlobalObject {
    pub fn new(
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
}

use JsMethodKind::*;
use JsObjectType::*;

lazy_static! {
    static ref JS_GLOBAL_OBJECTS: Vec<JsGlobalObject> = vec![
        JsGlobalObject::new(
            "Array",
            &[
                ("from", Static, &[
                        &[Array],
                        &[Array, Function],
                        &[Array, Function, Any],
                    ]),
                ("fromAsync", Static, &[
                        &[Array],
                        &[Array, Function],
                        &[Array, Function, Any],
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
            ],
            &[("length", Instance)],
        )
    ];
}
