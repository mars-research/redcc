use rustc_data_structures::fx::FxHashSet;
use rustc_middle::mir::{LocalDecls, Place};
use rustc_middle::ty::{self, print::with_no_trimmed_paths, AdtDef, Ty, TyCtxt, TypeAndMut};
use rustc_span::symbol::sym;

pub fn place_contains_rref<'tcx>(place: Place<'tcx>, tcx: TyCtxt<'tcx>, local_decls: &LocalDecls<'tcx>) -> bool {
    let place_ty = local_decls[place.local].ty;

    if ty_is_rref(place_ty, tcx) {
        eprintln!("base is rref");
        return true;
    }

    for (base, elem) in place.iter_projections() {
        let proj_ty = base.ty(local_decls, tcx).projection_ty(tcx, elem).ty;

        with_no_trimmed_paths!({
            eprintln!("proj type: {:?}", proj_ty);
        });

        if ty_is_rref(proj_ty, tcx) {
            eprintln!("proj is rref");
            return true;
        }
    }

    false
}

fn ty_is_rref<'tcx>(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> bool {
    match ty.ty_adt_def() {
        Some(adt) => tcx.is_diagnostic_item(sym::RRef, adt.did()),
        _ => false,
    }
}

pub fn contains_rref<'tcx>(tcx: TyCtxt<'tcx>, t: Ty<'tcx>) -> bool {
    contains_rref_impl(tcx, t, &mut FxHashSet::default())
}

fn contains_rref_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    t: Ty<'tcx>,
    visited: &mut FxHashSet<Ty<'tcx>>,
) -> bool {
    if !visited.contains(&t) {
        visited.insert(t);

        match t.kind() {
            ty::Adt(adt, _) => {
                tcx.is_diagnostic_item(sym::RRef, adt.did())
                    || adt_fields_contain_rref(tcx, adt, visited)
            }
            ty::Array(base_ty, _) => contains_rref_impl(tcx, *base_ty, visited),
            ty::Slice(base_ty) => contains_rref_impl(tcx, *base_ty, visited),
            ty::Tuple(types) => types.iter().any(|ty| contains_rref_impl(tcx, ty, visited)),
            ty::RawPtr(TypeAndMut { ty, .. }) => contains_rref_impl(tcx, *ty, visited),
            ty::Ref(_, base_ty, _) => contains_rref_impl(tcx, *base_ty, visited),
            // FIXME: generics
            // specifically, this fails for adts and other generic things when RRef is a substitution
            _ => false,
        }
    } else {
        false
    }
}

fn adt_fields_contain_rref<'tcx>(
    tcx: TyCtxt<'tcx>,
    adt: &'tcx AdtDef<'tcx>,
    visited: &mut FxHashSet<Ty<'tcx>>,
) -> bool {
    adt.variants()
        .iter()
        .any(|v| v.fields.iter().any(|f| contains_rref_impl(tcx, tcx.type_of(f.did), visited)))
}
