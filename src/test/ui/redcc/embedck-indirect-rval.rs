// build-pass

#![feature(rustc_attrs)]

#[cfg_attr(not(test), rustc_diagnostic_item = "RRef")]
struct RRef<T>(T);
struct Direct(RRef<i32>);
struct Indirect(Direct);

// Detect trivial embedding and non-embedding assignments
fn main() {
    // FIXME: i think it needs to detect the embedding here too
    let mut i = RRef(Indirect(Direct(RRef(1))));
    i.0.0 = Direct(RRef(2));
}
