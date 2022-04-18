use rustc_data_structures::fx::FxHashSet;
use rustc_middle::mir::{LocalDecls, Place, PlaceElem, PlaceRef};
use rustc_middle::ty::{self, AdtDef, Ty, TyCtxt, TypeAndMut};
use rustc_span::symbol::sym;

pub fn place_contains_rref<'tcx>(
    place: Place<'tcx>,
    tcx: TyCtxt<'tcx>,
    local_decls: &LocalDecls<'tcx>,
) -> bool {
    let place_ty = place_base_ty(place, local_decls);

    if ty_is_rref(place_ty, tcx) {
        return true;
    }

    for (base, elem) in place.iter_projections() {
        let proj_ty = place_projection_ty(base, elem, tcx, local_decls);

        if ty_is_rref(proj_ty, tcx) {
            return true;
        }
    }

    false
}

fn place_base_ty<'tcx>(place: Place<'tcx>, local_decls: &LocalDecls<'tcx>) -> Ty<'tcx> {
    local_decls[place.local].ty
}

fn place_projection_ty<'tcx>(
    base: PlaceRef<'tcx>,
    elem: PlaceElem<'tcx>,
    tcx: TyCtxt<'tcx>,
    local_decls: &LocalDecls<'tcx>,
) -> Ty<'tcx> {
    base.ty(local_decls, tcx).projection_ty(tcx, elem).ty
}

fn ty_is_rref<'tcx>(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> bool {
    match ty.ty_adt_def() {
        Some(adt) => tcx.is_diagnostic_item(sym::RRef, adt.did()),
        _ => false,
    }
}

pub fn ty_contains_rref<'tcx>(tcx: TyCtxt<'tcx>, t: Ty<'tcx>) -> bool {
    ty_contains_rref_impl(tcx, t, &mut FxHashSet::default())
}

fn ty_contains_rref_impl<'tcx>(
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
            ty::Array(base_ty, _) => ty_contains_rref_impl(tcx, *base_ty, visited),
            ty::Slice(base_ty) => ty_contains_rref_impl(tcx, *base_ty, visited),
            ty::Tuple(types) => types.iter().any(|ty| ty_contains_rref_impl(tcx, ty, visited)),
            ty::RawPtr(TypeAndMut { ty, .. }) => ty_contains_rref_impl(tcx, *ty, visited),
            ty::Ref(_, base_ty, _) => ty_contains_rref_impl(tcx, *base_ty, visited),
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
        .any(|v| v.fields.iter().any(|f| ty_contains_rref_impl(tcx, tcx.type_of(f.did), visited)))
}
