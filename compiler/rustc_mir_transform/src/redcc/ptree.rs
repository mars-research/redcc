use rustc_middle::{
    mir::{Field, Place, ProjectionElem},
    ty::{Const, Ty, TyCtxt},
};

#[derive(Clone, Debug)]
pub enum PTreeNode<'tcx> {
    Array(Box<PTreeNode<'tcx>>, Const<'tcx>),
    // Slice(Box<PTreeNode>), // PROBLEM: slices don't know how long they are, so need to emit a loop here
    Deref(Box<PTreeNode<'tcx>>),
    Fields(Vec<(PTreeNode<'tcx>, Field, Ty<'tcx>)>),
    RRef, // FIXME(todo): handle case where RRef base type also has RRefs inside it (need to add a node for it)
          // FIXME(todo): probably need to add something for enums here
}

impl<'tcx> PTreeNode<'tcx> {
    pub fn traverse(&self, place: Place<'tcx>, tcx: TyCtxt<'tcx>, f: impl Fn(Place<'tcx>) + Copy) {
        match self {
            PTreeNode::Array(_base, _length) => {
                // for i in 0..length
                // make a new place by adding index i to the input place
                // traverse the base node at this indexed place
                // ok this is slightly problematic because Place requires its index projections
                // to be indexed by Locals, which i'm not sure if i can fabricate here
                // could also just emit a loop for arrays like for slices, which i think would be more doable without hacks
                todo!()
            }
            PTreeNode::Deref(base) => {
                // make new place by adding deref projection
                let deref_place = place.project_deeper(&[ProjectionElem::Deref], tcx);
                // traverse base node w/ deref place
                base.traverse(deref_place, tcx, f);
            }
            PTreeNode::Fields(fields) => {
                // for each field:
                // make new place by adding field projection to input place
                // traverse node for that field with its place
                for (child, field, ty) in fields.iter() {
                    let field_place =
                        place.clone().project_deeper(&[ProjectionElem::Field(*field, *ty)], tcx);

                    child.traverse(field_place, tcx, f);
                }
            }
            PTreeNode::RRef => {
                // apply function to input place
                f(place);
                // FIXME(todo): recur on base type if provided
                // main problem is i'm not sure how i should be accessing the base type
                // on the other hand, i could just treat RRef as any other data structure
                // then, to know when to run f on the input place, just check its type
                // i think this is what i should move toward later on, for robustness, but this hack is fine for now
            }
        }
    }
}
