use super::base;
use cubecl_core::prelude::*;
use std::marker::PhantomData;

use crate::matmul::components::batch::{CubeCountDispatch, CubeDispatch};
use crate::matmul::components::global::full_load::CyclicLoading;
use crate::matmul::components::stage::{self};
use crate::matmul::components::MatmulProblem;
use crate::matmul::components::{batch, global};
use crate::matmul::components::{tile, MatmulSelection};

pub struct StandardAlgorithm<TMM, Dispatch = batch::TransposedDispatch> {
    pub _tmm: PhantomData<TMM>,
    pub _dispatch: PhantomData<Dispatch>,
}

impl<TMM, Dispatch> base::Algorithm for StandardAlgorithm<TMM, Dispatch>
where
    TMM: tile::TileMatmulFamily,
    Dispatch: CubeDispatch + CubeCountDispatch,
{
    type TileMatmul = TMM;
    type StageMatmul = stage::multi_buffer::MultiBufferMatmulFamily<Self::TileMatmul>;
    type GlobalMatmul =
        global::full_load::FullLoadMatmulFamily<Self::StageMatmul, CyclicLoading, CyclicLoading>;

    type BatchMatmul = batch::one_to_one::OneToOneMatmulFamily<Self::GlobalMatmul, Dispatch>;
    type Selection = MatmulSelection;

    fn cube_dim(selection: &MatmulSelection) -> CubeDim {
        CubeDim::new(selection.plane_dim, selection.num_stagess.m, 1)
    }

    fn cube_count(selection: &MatmulSelection, problem: &MatmulProblem) -> CubeCount {
        let m_stage = selection.num_stagess.m * selection.tile.m;
        let n_stage = selection.num_stagess.n * selection.tile.n;
        let cubes_for_m = (problem.m as u32 + m_stage - 1) / m_stage;
        let cubes_for_n = (problem.n as u32 + n_stage - 1) / n_stage;

        Dispatch::cube_count(cubes_for_m, cubes_for_n, problem.num_batches() as u32)
    }

    fn advanced_config() -> crate::matmul::kernels::matmul::AdvancedConfig {
        crate::matmul::kernels::matmul::AdvancedConfig {
            lhs_tiling_order: stage::TilingOrderConfig::ColMajor,
            rhs_tiling_order: stage::TilingOrderConfig::RowMajor,
            enforced_tile_layout: (None, None),
        }
    }
}
