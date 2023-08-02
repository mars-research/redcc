mod embedck;
mod ptree;

use crate::MirPass;
use rustc_data_structures::fx::FxHashMap;
use rustc_index::IndexVec;
use rustc_middle::mir::{
    visit::MutVisitor, BasicBlock, BasicBlockData, Body, CallSource, Local, LocalDecl, Location,
    Operand, Place, Rvalue, Terminator, TerminatorKind, UnwindAction,
};
use rustc_middle::ty::{print::with_no_trimmed_paths, Ty, TyCtxt};

// a rough plan for doing the rewriting:
// figure out how function calls are represented in mir
// modify the rvalue rref detection to return a path to any rvalue it discovers
// IDEA: turn this rref path detection into an iterator, rather than allocating space for it
// that is, assuming this is both possible and ergonomic
// figure out how to insert a function call in mir
// insert a call to some kind of "canary" function that just prints a debug message
// implement and insert a call to a function that basically does what we want
// IDEA: embed the clear call post-assignment. basically, do the assignment, _use_ the assignment place to know
// where to start your traversal, and then somehow scan the rvalue type for where the RRef(s) are. you'll then need
// path detection figure out how to get to those RRef(s), but for each one you can then insert a call to the clear function.

// take special care for arrays and tuples and other situations where there may be multiple RRefs in a single type.

pub struct RRefEmbedCorrectionTransform;

impl<'tcx> MirPass<'tcx> for RRefEmbedCorrectionTransform {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
        // FIXME: figure this clone thing out
        let mut visitor = RRefEmbedCollectionVisitor {
            tcx,
            local_decls: body.local_decls.clone(),
            embed_map: FxHashMap::default(),
        };
        visitor.visit_body(body);

        let print_embed_def = if let Some(def) = tcx.lang_items().redcc_print_embed() {
            def
        } else {
            // FIXME: Add a warning?
            return;
        };

        if !visitor.embed_map.is_empty() {
            //let debug_str = tcx.def_path_debug_str(body.source.def_id());
            //eprintln!("Rewriting {:?}", debug_str);

            let unit_temp = body.get_unit_temp(tcx);
            let mut bbs = body.basic_blocks.as_mut();
            for (bb, embeds) in visitor.embed_map.drain() {
                let split_embeds = bbs.split_embeds(bb, embeds);

                for embed in split_embeds {
                    let (first, second) = embed.location;
                    let source_info = bbs[second].statements[0].source_info;

                    let print_embed =
                        Operand::function_handle(tcx, print_embed_def, [], source_info.span);

                    let terminator = Terminator {
                        source_info,
                        kind: TerminatorKind::Call {
                            func: print_embed,
                            args: vec![],
                            destination: unit_temp,
                            target: Some(second),
                            unwind: UnwindAction::Continue,
                            call_source: CallSource::Misc,
                            fn_span: source_info.span,
                        },
                    };
                    bbs[first].terminator = Some(terminator);
                }
            }
        }
    }
}

/// An RRef embedding.
///
/// The location is identified by either a `Location`
/// before the basic blocks are split or a
/// `(BasicBlock, BasicBlock)` afterwards.
#[allow(dead_code)]
#[derive(Debug)]
struct Embedding<'tcx, L> {
    rref_place_ty: Ty<'tcx>,
    rvalue_ty: Ty<'tcx>,
    place_ty: Ty<'tcx>,
    location: L,
}

/// An RRef embedding.
///
/// Before calls to the runtime can be injected, the
/// basic blocks they reside in have to be split.
///
/// The resulting embeddings after splitting are described by
/// `SplitEmbedding`s.
type OriginalEmbedding<'tcx> = Embedding<'tcx, Location>;

/// An RRef embedding.
///
/// The statement is at the beginning of the second
/// basic block. The two basic blocks are joined
/// together by `TerminatorKind::Goto`.
///
/// In this form, calls to the runtime can be easily
/// injected between the basic blocks.
type SplitEmbedding<'tcx> = Embedding<'tcx, (BasicBlock, BasicBlock)>;

struct RRefEmbedCollectionVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    local_decls: IndexVec<Local, LocalDecl<'tcx>>,
    embed_map: FxHashMap<BasicBlock, Vec<OriginalEmbedding<'tcx>>>,
}

impl<'tcx> MutVisitor<'tcx> for RRefEmbedCollectionVisitor<'tcx> {
    fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_assign(
        &mut self,
        place: &mut Place<'tcx>,
        rvalue: &mut Rvalue<'tcx>,
        location: Location,
    ) {
        let place_ty = place.ty(&self.local_decls, self.tcx).ty;
        let rvalue_ty = rvalue.ty(&self.local_decls, self.tcx);

        // if embedck::place_contains_embedded_rref(*place, self.tcx, &self.local_decls)
        //     && embedck::ty_contains_rref(self.tcx, rvalue_ty)
        // {
        //     with_no_trimmed_paths!({
        //         println!("EMBED {} -> {}", rvalue_ty, place_ty);
        //     });
        // }

        if embedck::place_contains_embedded_rref(*place, self.tcx, &self.local_decls) {
            if let Some(node) = embedck::locate_embedded_rrefs(self.tcx, rvalue_ty) {
                node.traverse(*place, self.tcx, &mut |place| {
                    let rref_place_ty = place.ty(&self.local_decls, self.tcx).ty;

                    let embeds = self.embed_map.entry(location.block).or_default();
                    embeds.push(Embedding { rref_place_ty, rvalue_ty, place_ty, location });

                    with_no_trimmed_paths!({
                        println!("EMBED {} in {} -> {}", rref_place_ty, rvalue_ty, place_ty);
                    })
                });
            }
        }
    }
}

trait BasicBlocksMutExt<'tcx> {
    /// Splits a basic block at a statement.
    ///
    /// Everything _before_ the statement is kept in place, with
    /// the rest moved to a new basic block.
    fn split_block_at(&mut self, location: Location) -> (BasicBlock, BasicBlock);

    /// Split a list of embeddings in a basic block.
    fn split_embeds(
        &mut self,
        bb: BasicBlock,
        embeds: Vec<OriginalEmbedding<'tcx>>,
    ) -> Vec<SplitEmbedding<'tcx>>;
}

impl<'tcx> BasicBlocksMutExt<'tcx> for &mut IndexVec<BasicBlock, BasicBlockData<'tcx>> {
    fn split_block_at(&mut self, location: Location) -> (BasicBlock, BasicBlock) {
        let first = location.block;
        let (second, split_point_src) = {
            let data = &mut self[first];
            let split_point_src = data.statements[location.statement_index].source_info;
            let rest_statements = data.statements.split_off(location.statement_index);

            let mut second = BasicBlockData::new(data.terminator.take());
            second.statements = rest_statements;

            (self.push(second), split_point_src)
        };

        let terminator = Terminator {
            source_info: split_point_src,
            kind: TerminatorKind::Goto { target: second },
        };
        self[location.block].terminator = Some(terminator);

        (first, second)
    }

    fn split_embeds(
        &mut self,
        bb: BasicBlock,
        mut embeds: Vec<OriginalEmbedding<'tcx>>,
    ) -> Vec<SplitEmbedding<'tcx>> {
        embeds.sort_unstable_by(|a, b| a.location.statement_index.cmp(&b.location.statement_index));

        let mut cur_bb = bb;
        let mut cur_offset = 0;

        embeds
            .into_iter()
            .map(|embed| {
                //eprintln!("{:?}: {:#?}", bb, embed);

                let split_loc = Location {
                    block: cur_bb,
                    statement_index: embed.location.statement_index - cur_offset,
                };

                let (first, second) = self.split_block_at(split_loc);

                cur_bb = second;
                cur_offset = embed.location.statement_index;

                //eprintln!("Split embed at {:?} into {:?}, {:?}", split_loc, first, second);

                Embedding {
                    rref_place_ty: embed.rref_place_ty,
                    rvalue_ty: embed.rvalue_ty,
                    place_ty: embed.place_ty,
                    location: (first, second),
                }
            })
            .collect()
    }
}

trait BodyExt<'tcx> {
    /// Returns a location for the unit temporary.
    ///
    /// See also `rustc_mir_build::Builder::get_unit_temp`.
    fn get_unit_temp(&mut self, tcx: TyCtxt<'tcx>) -> Place<'tcx>;
}

impl<'tcx> BodyExt<'tcx> for Body<'tcx> {
    fn get_unit_temp(&mut self, tcx: TyCtxt<'tcx>) -> Place<'tcx> {
        let ty = Ty::new_unit(tcx);
        if let Some(pair) =
            self.local_decls.iter_enumerated().find(|(_local, decl)| decl.ty == ty && decl.internal)
        {
            return Place::from(pair.0);
        }

        let local = self.local_decls.push(LocalDecl::new(ty, self.span).internal());
        Place::from(local)
    }
}
