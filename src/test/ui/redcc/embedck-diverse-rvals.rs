// build-pass

#![feature(rustc_attrs)]

#[cfg_attr(not(test), rustc_diagnostic_item = "RRef")]
struct RRef<T>(T);
struct Direct(RRef<i32>);
struct Ref<'a>(&'a RRef<i32>);

enum MaybeRRef {
    Yes(RRef<i32>),
    No,
}

fn main() {
    // array
    // let mut array = RRef([RRef(0), RRef(1)]);
    // array.0 = [RRef(1), RRef(2)];
    // FIXME: arrays don't work yet

    // tuple
    let mut tuple = RRef((RRef(0), RRef(1)));
    tuple.0 = (RRef(0), RRef(1));

    // reference
    let tmp = &RRef(1);
    let mut r = RRef(Ref(tmp));
    r.0.0 = &RRef(0);

    // enum
    // FIXME: enums are an unhandled special case for now
    // let mut m = RRef(MaybeRRef::No);
    // m.0 = MaybeRRef::Yes(RRef(1));

    // raw pointer
    let p: *const RRef<_> = &RRef(1);
    let mut r = RRef(p);
    r.0 = &RRef(2);
}
