
fn main() {
    cc::Build::new()
    .file("fuzzilli/Sources/libcoverage/coverage.c")
    .include("fuzzilli/Sources/libcoverage/include")
    .compile("coverage");
}