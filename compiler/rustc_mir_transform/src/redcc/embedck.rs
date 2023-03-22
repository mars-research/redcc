// use rustc_data_structures::stable_set::FxHashSet;
use rustc_data_structures::stable_map::FxHashMap;
use rustc_index::vec::Idx;
use rustc_middle::mir::{Field, LocalDecls, Place, PlaceElem, PlaceRef};
use rustc_middle::ty::{self, AdtDef, List, Ty, TyCtxt, TypeAndMut};
use rustc_span::symbol::sym;

use super::ptree::PTreeNode;

pub fn place_contains_embedded_rref<'tcx>(
    place: Place<'tcx>,
    tcx: TyCtxt<'tcx>,
    local_decls: &LocalDecls<'tcx>,
) -> bool {
    let place_ty = place_base_ty(place, local_decls);

    // an embedding must have projections
    // if it has no projections, then the type of source and dest must match exactly
    // and there can be _no_ embedding

    let place_base_is_embedding_rref = ty_is_rref(place_ty, tcx) && !place.projection.is_empty();

    place_base_is_embedding_rref
        || place.iter_projections().rev().skip(1).any(|(base, elem)| {
            let proj_ty = place_projection_ty(base, elem, tcx, local_decls);

            ty_is_rref(proj_ty, tcx)
        })
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

pub fn locate_embedded_rrefs<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<PTreeNode<'tcx>> {
    locate_embedded_rrefs_impl(tcx, ty, &mut FxHashMap::default())
}

fn locate_embedded_rrefs_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    t: Ty<'tcx>,
    visited: &mut FxHashMap<Ty<'tcx>, Option<PTreeNode<'tcx>>>,
) -> Option<PTreeNode<'tcx>> {
    match visited.get(&t) {
        Some(existing_node) => existing_node.clone(),
        None => {
            let node = match t.kind() {
                ty::Adt(adt, _) => {
                    if tcx.is_diagnostic_item(sym::RRef, adt.did()) {
                        Some(PTreeNode::RRef)
                    } else {
                        locate_rrefs_in_adt_fields(tcx, *adt, visited)
                    }
                }

                ty::Array(base_ty, length) => locate_embedded_rrefs_impl(tcx, *base_ty, visited)
                    .map(|node| PTreeNode::Array(Box::new(node), *length)),

                ty::Ref(_, base_ty, _) | ty::RawPtr(TypeAndMut { ty: base_ty, .. }) => {
                    locate_embedded_rrefs_impl(tcx, *base_ty, visited)
                        .map(|node| PTreeNode::Deref(Box::new(node)))
                }

                ty::Tuple(types) => locate_rrefs_in_tuple_fields(tcx, types, visited),

                ty::Slice(..) => todo!(),
                _ => None,
            };

            visited.insert(t, node.clone());
            node
        }
    }
}

fn locate_rrefs_in_adt_fields<'tcx>(
    tcx: TyCtxt<'tcx>,
    adt: AdtDef<'tcx>,
    visited: &mut FxHashMap<Ty<'tcx>, Option<PTreeNode<'tcx>>>,
) -> Option<PTreeNode<'tcx>> {
    if !adt.is_struct() {
        None // FIXME: todo
    } else {
        locate_rrefs_in_fields(tcx, adt.all_fields().map(|f| tcx.type_of(f.did)), visited)
    }
}

fn locate_rrefs_in_tuple_fields<'tcx>(
    tcx: TyCtxt<'tcx>,
    field_types: &List<Ty<'tcx>>,
    visited: &mut FxHashMap<Ty<'tcx>, Option<PTreeNode<'tcx>>>,
) -> Option<PTreeNode<'tcx>> {
    locate_rrefs_in_fields(tcx, field_types.iter(), visited)
}

fn locate_rrefs_in_fields<'tcx>(
    tcx: TyCtxt<'tcx>,
    field_types: impl Iterator<Item = Ty<'tcx>>,
    visited: &mut FxHashMap<Ty<'tcx>, Option<PTreeNode<'tcx>>>,
) -> Option<PTreeNode<'tcx>> {
    let field_nodes = field_types
        .enumerate()
        .filter_map(|(i, ty)| {
            locate_embedded_rrefs_impl(tcx, ty, visited).map(|node| (node, Field::new(i), ty))
        })
        .collect::<Vec<_>>();

    if field_nodes.is_empty() { None } else { Some(PTreeNode::Fields(field_nodes)) }
}

// NOTE: this is old, keeping it around as a reference

/*
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
                    || adt_fields_contain_rref(tcx, *adt, visited)
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
    adt: AdtDef<'tcx>,
    visited: &mut FxHashSet<Ty<'tcx>>,
) -> bool {
    // FIXME(todo): remove struct restriction to handle enums
    adt.is_struct() && adt.all_fields()
        .any(|f| ty_contains_rref_impl(tcx, tcx.type_of(f.did), visited))
}
*/
