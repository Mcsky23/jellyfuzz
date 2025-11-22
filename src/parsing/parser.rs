use swc_common;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_codegen::Emitter;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_parser::{self, parse_file_as_script};
use swc_ecma_parser::{EsSyntax, Syntax};
use swc_ecma_visit::swc_ecma_ast::{EsVersion, Script};

pub fn parse_js(src: String) -> anyhow::Result<Script> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("simz.js".into()).into(), src);
    let syntax = Syntax::Es(EsSyntax {
        jsx: false, // set true if you want JSX
        decorators: false,
        decorators_before_export: false,
        export_default_from: true,
        import_attributes: true,
        ..Default::default()
    });

    let mut errs = vec![];
    let aux = parse_file_as_script(&fm, syntax, EsVersion::Es2024, None, &mut errs)
        .map_err(|e| anyhow::anyhow!("error parsing script: {:?}", e));
    aux
}

pub fn generate_js(script: Script) -> anyhow::Result<Vec<u8>> {
    let cm = Lrc::new(SourceMap::default());
    let mut out = Vec::new();
    let wr = JsWriter::new(cm.clone(), "\n", &mut out, None);
    let mut emitter = Emitter {
        cfg: Default::default(),
        comments: None, // remove comments so we dont have to build another mutator
        cm,
        wr,
    };
    emitter.emit_script(&script)?;

    // support natives syntax
    // TODO: this is wacky for now
    let result = String::from_utf8_lossy(&out);
    let result = result.replace("<invalid> % ", "%");
    out = result.into_bytes();
    Ok(out)
}
